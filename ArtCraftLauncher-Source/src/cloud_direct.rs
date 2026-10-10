//! Cloud screen state and asynchronous direct Google Drive operations.
use super::*;
use crate::google_drive::{self, Account, Drive, Version};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupTarget {
    #[default]
    Google,
    Nas,
}
impl BackupTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Google => "Google Drive",
            Self::Nas => "Local NAS",
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub client_id: String,
    pub account: Option<Account>,
    pub selected: Vec<PathBuf>,
    pub automatic: bool,
    pub versions: Vec<Version>,
    pub confirmed: HashMap<PathBuf, (u64, String)>,
    pub target: BackupTarget,
    pub nas: nas_backup::Settings,
}
#[derive(Clone)]
pub struct Notice {
    pub id: u64,
    pub title: String,
    pub detail: String,
    pub progress: f32,
    pub finished: Option<Instant>,
    pub failed: bool,
}
#[derive(Clone)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Checking,
    Connected,
    Disconnecting,
    Error(String),
}
impl ConnectionState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Disconnected => "Not connected",
            Self::Connecting => "Connecting…",
            Self::Checking => "Checking connection…",
            Self::Connected => "Connected",
            Self::Disconnecting => "Disconnecting…",
            Self::Error(_) => "Connection failed",
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    Connect,
    Check,
    Disconnect,
    Transfer,
    NasConnect,
    NasCheck,
}
enum Event {
    Connected(Account),
    Verified,
    ConnectionError(String),
    Disconnected,
    Versions(Vec<Version>),
    NasConnected(nas_backup::Settings),
    NasVerified,
    NasError(String),
    NasVersions(Vec<Version>),
    Uploaded(BackupTarget, PathBuf, u64, Version),
    FileError(PathBuf, String),
    Progress(String, f32),
    Finished(Result<String, String>),
}
pub struct DirectCloud {
    pub settings: Settings,
    pub connection: ConnectionState,
    pub nas_connection: ConnectionState,
    pub library: cloud_layout::Library,
    pub kind_filter: String,
    pub busy: bool,
    pub message: String,
    pub search: String,
    pub tab: u8,
    pub app_filter: String,
    pub list_view: bool,
    pub disconnect_open: bool,
    pub errors: HashMap<PathBuf, String>,
    pub notices: Vec<Notice>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    active_notice: Option<u64>,
    next_notice: u64,
    last_auto: Instant,
    save_error: Option<String>,
    refresh_pending: bool,
    check_pending: bool,
    last_check: Instant,
    operation: Option<Operation>,
    nas_check_pending: bool,
    nas_last_check: Instant,
}
fn checked_drive(tx: &Sender<Event>, account: &Account) -> Result<Drive, String> {
    match Drive::connect(account) {
        Ok(drive) => {
            let _ = tx.send(Event::Verified);
            Ok(drive)
        }
        Err(error) => {
            let _ = tx.send(Event::ConnectionError(error.clone()));
            Err(error)
        }
    }
}
fn settings_path() -> Result<PathBuf, String> {
    Ok(app_data()
        .ok_or("App data folder unavailable.")?
        .join("cloud/direct-google.json"))
}
impl DirectCloud {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let mut message = String::new();
        let mut settings: Settings = match settings_path().ok().filter(|p| p.exists()) {
            Some(path) => match fs::read(path)
                .ok()
                .and_then(|v| serde_json::from_slice(&v).ok())
            {
                Some(settings) => settings,
                None => {
                    message = "Cloud settings could not be read. Reconnect Google Drive and select your files.".into();
                    Settings::default()
                }
            },
            None => Settings::default(),
        };
        let mut connection = if settings.account.is_some() {
            ConnectionState::Checking
        } else {
            ConnectionState::Disconnected
        };
        if settings.account.is_some() && settings.client_id != google_drive::CLIENT_ID {
            message = "Google's app configuration has changed. Connect Google Drive again.".into();
            connection = ConnectionState::Error(message.clone());
            settings.account = None;
            settings.automatic = false;
            settings.versions.clear();
            settings.confirmed.clear();
        }
        let check_pending = settings.account.is_some();
        let nas_check_pending = settings.nas.root.is_some();
        Self {
            settings,
            connection,
            nas_connection: if nas_check_pending {
                ConnectionState::Checking
            } else {
                ConnectionState::Disconnected
            },
            library: cloud_layout::Library::default(),
            kind_filter: String::new(),
            busy: false,
            message,
            search: String::new(),
            tab: 0,
            app_filter: String::new(),
            list_view: false,
            disconnect_open: false,
            errors: HashMap::new(),
            notices: Vec::new(),
            tx,
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
            active_notice: None,
            next_notice: 0,
            last_auto: Instant::now(),
            save_error: None,
            refresh_pending: false,
            check_pending,
            last_check: Instant::now(),
            operation: None,
            nas_check_pending,
            nas_last_check: Instant::now(),
        }
    }
    pub(super) fn persist(&self) -> Result<(), String> {
        let path = settings_path()?;
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let temp = path.with_extension("new");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(&self.settings).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temp, path).map_err(|e| e.to_string())
    }
    pub fn save(&mut self) {
        match self.persist() {
            Ok(()) => self.save_error = None,
            Err(error) => {
                self.settings.automatic = false;
                self.message = format!("Could not save cloud settings: {error}");
                self.save_error = Some(self.message.clone());
            }
        }
    }
    fn begin(
        &mut self,
        message: &str,
        operation: Operation,
    ) -> Option<(Sender<Event>, Arc<AtomicBool>)> {
        if self.busy {
            return None;
        }
        self.busy = true;
        self.operation = Some(operation);
        self.message = message.into();
        self.cancel = Arc::new(AtomicBool::new(false));
        Some((self.tx.clone(), self.cancel.clone()))
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionState::Connected) && self.settings.account.is_some()
    }
    pub fn ready(&self) -> bool {
        match self.settings.target {
            BackupTarget::Google => self.is_connected(),
            BackupTarget::Nas => {
                matches!(self.nas_connection, ConnectionState::Connected)
                    && self.settings.nas.root.is_some()
            }
        }
    }
    pub fn target_label(&self) -> &'static str {
        self.settings.target.label()
    }
    pub fn select_target(&mut self, target: BackupTarget) {
        if self.busy || self.settings.target == target {
            return;
        }
        self.settings.target = target;
        self.settings.automatic = false;
        self.errors.clear();
        self.message.clear();
        self.save();
        self.check_on_open();
    }
    pub fn folder_label(&self, project: &Project) -> String {
        self.library
            .destinations
            .get(&project.path)
            .map(|d| d.display())
            .unwrap_or_default()
    }
    pub fn saved_versions(&self) -> &[Version] {
        match self.settings.target {
            BackupTarget::Google => &self.settings.versions,
            BackupTarget::Nas => &self.settings.nas.versions,
        }
    }
    pub fn connect_nas(&mut self, folder: PathBuf) {
        let Some((tx, cancel)) = self.begin("Connecting to NAS…", Operation::NasConnect) else {
            return;
        };
        self.nas_connection = ConnectionState::Connecting;
        thread::spawn(move || {
            let result = nas_backup::connect(&folder).and_then(|settings| {
                google_drive::cancelled(&cancel)?;
                let _ = tx.send(Event::NasConnected(settings));
                Ok("NAS connected. Select files and click Sync.".into())
            });
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn check_nas(&mut self) {
        if self.settings.nas.root.is_none() {
            return;
        }
        let Some((tx, cancel)) = self.begin("Checking NAS access…", Operation::NasCheck) else {
            return;
        };
        let settings = self.settings.nas.clone();
        self.nas_connection = ConnectionState::Checking;
        self.nas_check_pending = false;
        thread::spawn(move || {
            let result = nas_backup::check(&settings).and_then(|_| {
                google_drive::cancelled(&cancel)?;
                let _ = tx.send(Event::NasVerified);
                Ok("NAS connection verified.".into())
            });
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn disconnect_nas(&mut self) {
        if self.busy {
            return;
        }
        self.settings.nas = nas_backup::Settings::default();
        self.nas_connection = ConnectionState::Disconnected;
        self.nas_check_pending = false;
        if self.settings.target == BackupTarget::Nas {
            self.settings.automatic = false;
        }
        self.message = "NAS disconnected. Existing backups were kept.".into();
        self.save();
    }
    pub fn check_connection(&mut self) {
        let Some(account) = self.settings.account.clone() else {
            return;
        };
        let Some((tx, cancel)) = self.begin("Checking Google Drive access…", Operation::Check)
        else {
            return;
        };
        self.connection = ConnectionState::Checking;
        self.check_pending = false;
        thread::spawn(move || {
            let result = (|| {
                google_drive::cancelled(&cancel)?;
                checked_drive(&tx, &account)?;
                google_drive::cancelled(&cancel)?;
                Ok("Google Drive connection verified.".into())
            })();
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn check_on_open(&mut self) {
        if self.settings.account.is_some() && self.last_check.elapsed() >= Duration::from_secs(30) {
            self.check_pending = true;
        }
        if self.settings.nas.root.is_some()
            && self.nas_last_check.elapsed() >= Duration::from_secs(30)
        {
            self.nas_check_pending = true;
        }
    }
    pub fn connect(&mut self) {
        let Some((tx, cancel)) =
            self.begin("Finish Google sign-in in your browser…", Operation::Connect)
        else {
            return;
        };
        self.connection = ConnectionState::Connecting;
        self.check_pending = false;
        thread::spawn(move || {
            let result = (|| {
                let account = google_drive::sign_in(&cancel)?;
                // Verify the stored refresh credential and live Drive access before reporting success.
                Drive::connect(&account)?;
                google_drive::cancelled(&cancel)?;
                let _ = tx.send(Event::Connected(account));
                Ok("Google Drive connected. Select files and click Sync.".into())
            })();
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn disconnect(&mut self) {
        let Some((tx, _)) = self.begin("Disconnecting Google Drive…", Operation::Disconnect)
        else {
            return;
        };
        self.connection = ConnectionState::Disconnecting;
        thread::spawn(move || {
            let result = google_drive::disconnect().map(|()| {
                let _ = tx.send(Event::Disconnected);
                "Google Drive disconnected. Uploaded files were kept.".into()
            });
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn refresh(&mut self) {
        if self.settings.target == BackupTarget::Nas {
            let settings = self.settings.nas.clone();
            let Some((tx, cancel)) = self.begin("Reading NAS backups…", Operation::Transfer)
            else {
                return;
            };
            thread::spawn(move || {
                let result = nas_backup::versions(&settings, &cancel).map(|files| {
                    let _ = tx.send(Event::NasVerified);
                    let _ = tx.send(Event::NasVersions(files));
                    "NAS backups refreshed.".into()
                });
                if let Err(error) = &result {
                    let _ = tx.send(Event::NasError(error.clone()));
                }
                let _ = tx.send(Event::Finished(result));
            });
            return;
        }
        let Some(account) = self.settings.account.clone() else {
            return;
        };
        let Some((tx, cancel)) = self.begin("Reading Google Drive…", Operation::Transfer) else {
            return;
        };
        thread::spawn(move || {
            let result = checked_drive(&tx, &account)
                .and_then(|drive| drive.versions(&cancel))
                .map(|files| {
                    let _ = tx.send(Event::Versions(files));
                    "Cloud files refreshed.".into()
                });
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn restore(&mut self, version: Version, destination: PathBuf) {
        if self.settings.target == BackupTarget::Nas {
            let settings = self.settings.nas.clone();
            let Some((tx, cancel)) = self.begin("Restoring NAS version…", Operation::Transfer)
            else {
                return;
            };
            thread::spawn(move || {
                let result = nas_backup::restore(&settings, &version, &destination, &cancel)
                    .map(|()| "Saved a verified, separate copy of your file.".into());
                let _ = tx.send(Event::Finished(result));
            });
            return;
        }
        let Some(account) = self.settings.account.clone() else {
            return;
        };
        let Some((tx, cancel)) = self.begin("Downloading saved version…", Operation::Transfer)
        else {
            return;
        };
        thread::spawn(move || {
            let result = checked_drive(&tx, &account)
                .and_then(|drive| drive.restore(&version, &destination, &cancel))
                .map(|()| "Saved a verified, separate copy of your project.".into());
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn status(&self, project: &Project) -> &'static str {
        if self.errors.contains_key(&project.path) {
            return "Needs attention";
        }
        let (configured, confirmed) = match self.settings.target {
            BackupTarget::Google => (self.settings.account.is_some(), &self.settings.confirmed),
            BackupTarget::Nas => (
                self.settings.nas.root.is_some(),
                &self.settings.nas.confirmed,
            ),
        };
        if !configured {
            return "Local only";
        }
        if confirmed.get(&project.path).is_some_and(|(revision, id)| {
            *revision == project.revision
                && self.saved_versions().iter().any(|v| {
                    &v.id == id
                        && self
                            .library
                            .destinations
                            .get(&project.path)
                            .is_some_and(|d| d.key() == v.destination)
                })
        }) {
            return "Backed up";
        }
        if self.settings.selected.contains(&project.path) {
            "Ready to sync"
        } else {
            "Local only"
        }
    }
    pub fn sync(&mut self) {
        if !self.ready() {
            self.message = format!(
                "Verify or reconnect {} before syncing.",
                self.target_label()
            );
            return;
        }
        let account = self.settings.account.clone();
        let nas = self.settings.nas.clone();
        let target = self.settings.target;
        let projects: Vec<_> = self
            .library
            .files
            .iter()
            .filter(|p| self.settings.selected.contains(&p.path))
            .filter_map(|p| {
                self.library
                    .destinations
                    .get(&p.path)
                    .map(|d| (p.clone(), d.clone()))
            })
            .collect();
        if projects.is_empty() {
            self.message = "Select files to sync.".into();
            return;
        }
        if target == BackupTarget::Nas {
            let mut destinations = std::collections::HashSet::new();
            for (project, destination) in &projects {
                if !destinations
                    .insert(format!("{}{}", destination.display(), project.title).to_lowercase())
                {
                    self.message = format!(
                        "Two selected files would use {}{}. Rename one file or keep them in separate subfolders before backing up to NAS.",
                        destination.display(),
                        project.title
                    );
                    return;
                }
            }
        }
        let Some((tx, cancel)) = self.begin("Starting sync…", Operation::Transfer) else {
            return;
        };
        self.errors.clear();
        self.next_notice += 1;
        self.active_notice = Some(self.next_notice);
        self.notices.push(Notice {
            id: self.next_notice,
            title: format!("Syncing to {}", target.label()),
            detail: "Preparing files…".into(),
            progress: 0.0,
            finished: None,
            failed: false,
        });
        thread::spawn(move || {
            let result = (|| {
                let total = projects.len();
                let mut failed = 0;
                for (index, (project, destination)) in projects.iter().enumerate() {
                    google_drive::cancelled(&cancel)?;
                    // Refresh credentials for every file so a long batch can outlive an access token.
                    let progress = |sent: u64, bytes: u64, phase: &str| {
                        let fraction = if bytes == 0 {
                            0.0
                        } else {
                            sent as f32 / bytes as f32
                        };
                        let progress = (index as f32 + fraction.min(0.98)) / total as f32;
                        let _ = tx.send(Event::Progress(
                            format!("{} · {} of {} · {}", project.title, index + 1, total, phase),
                            progress,
                        ));
                    };
                    let uploaded = match target {
                        BackupTarget::Google => checked_drive(
                            &tx,
                            account.as_ref().ok_or("Connect Google Drive first.")?,
                        )?
                        .upload(project, destination, &cancel, progress),
                        BackupTarget::Nas => {
                            nas_backup::backup(&nas, project, destination, &cancel, progress)
                        }
                    };
                    match uploaded {
                        Ok(saved) => {
                            let _ = tx.send(Event::Uploaded(
                                target,
                                project.path.clone(),
                                project.revision,
                                saved,
                            ));
                        }
                        Err(error) => {
                            failed += 1;
                            if target == BackupTarget::Nas {
                                if let Err(connection_error) = nas_backup::check(&nas) {
                                    let _ = tx.send(Event::NasError(connection_error));
                                }
                            }
                            let _ = tx.send(Event::FileError(project.path.clone(), error));
                        }
                    }
                    google_drive::cancelled(&cancel)?;
                }
                if failed == 0 {
                    Ok(format!(
                        "{} file{} backed up to {}.",
                        total,
                        if total == 1 { "" } else { "s" },
                        target.label()
                    ))
                } else {
                    Err(format!(
                        "{} of {} files backed up. {} need attention; retry Sync.",
                        total - failed,
                        total,
                        failed
                    ))
                }
            })();
            let _ = tx.send(Event::Finished(result));
        });
    }
    pub fn tick(&mut self, automatic_allowed: bool, cloud_visible: bool) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::NasConnected(mut settings) => {
                    if self.settings.nas.identity == settings.identity {
                        settings.versions = self.settings.nas.versions.clone();
                        settings.confirmed = self.settings.nas.confirmed.clone();
                    }
                    self.settings.nas = settings;
                    self.settings.target = BackupTarget::Nas;
                    self.settings.automatic = false;
                    self.nas_connection = ConnectionState::Connected;
                    self.nas_last_check = Instant::now();
                    self.nas_check_pending = false;
                    self.errors.clear();
                    self.save();
                    self.refresh_pending = true;
                }
                Event::NasVerified => {
                    self.nas_connection = ConnectionState::Connected;
                    self.nas_last_check = Instant::now();
                    self.nas_check_pending = false;
                }
                Event::NasError(error) => {
                    self.nas_connection = ConnectionState::Error(error);
                    self.nas_last_check = Instant::now();
                }
                Event::NasVersions(files) => {
                    self.settings
                        .nas
                        .confirmed
                        .retain(|_, (_, id)| files.iter().any(|v| &v.id == id));
                    self.settings.nas.versions = files;
                    self.save();
                }
                Event::Connected(account) => {
                    if self.settings.account.as_ref().map(|a| &a.id) != Some(&account.id) {
                        self.settings.versions.clear();
                        self.settings.confirmed.clear();
                    }
                    self.settings.account = Some(account);
                    self.settings.client_id = google_drive::CLIENT_ID.into();
                    self.connection = ConnectionState::Connected;
                    self.last_check = Instant::now();
                    self.errors.clear();
                    self.save();
                    self.refresh_pending = true;
                }
                Event::Verified => {
                    self.connection = ConnectionState::Connected;
                    self.last_check = Instant::now();
                    self.check_pending = false;
                }
                Event::ConnectionError(error) => {
                    self.connection = ConnectionState::Error(error);
                    self.last_check = Instant::now();
                }
                Event::Disconnected => {
                    self.refresh_pending = false;
                    self.check_pending = false;
                    self.connection = ConnectionState::Disconnected;
                    self.settings.account = None;
                    if self.settings.target == BackupTarget::Google {
                        self.settings.automatic = false;
                    }
                    self.settings.versions.clear();
                    self.settings.confirmed.clear();
                    self.errors.clear();
                    self.save();
                }
                Event::Versions(files) => {
                    self.settings
                        .confirmed
                        .retain(|_, (_, id)| files.iter().any(|v| &v.id == id));
                    self.settings.versions = files;
                    self.save();
                }
                Event::Uploaded(target, path, revision, saved) => {
                    self.errors.remove(&path);
                    let (confirmed, versions) = match target {
                        BackupTarget::Google => {
                            (&mut self.settings.confirmed, &mut self.settings.versions)
                        }
                        BackupTarget::Nas => (
                            &mut self.settings.nas.confirmed,
                            &mut self.settings.nas.versions,
                        ),
                    };
                    confirmed.insert(path, (revision, saved.id.clone()));
                    versions.retain(|v| v.id != saved.id);
                    versions.insert(0, saved);
                    if target == BackupTarget::Nas {
                        self.nas_connection = ConnectionState::Connected;
                        self.nas_last_check = Instant::now();
                    }
                    self.save();
                }
                Event::FileError(path, error) => {
                    self.errors.insert(path, error);
                }
                Event::Progress(detail, progress) => {
                    self.message = detail.clone();
                    if let Some(notice) = self
                        .notices
                        .iter_mut()
                        .find(|n| Some(n.id) == self.active_notice)
                    {
                        notice.detail = detail;
                        notice.progress = notice.progress.max(progress);
                    }
                }
                Event::Finished(result) => {
                    self.busy = false;
                    let operation = self.operation.take();
                    if !matches!(operation, Some(Operation::Check | Operation::NasCheck)) {
                        self.last_auto = Instant::now();
                    }
                    if let Err(error) = &result {
                        if matches!(operation, Some(Operation::NasConnect | Operation::NasCheck)) {
                            self.nas_connection = ConnectionState::Error(error.clone());
                            self.nas_last_check = Instant::now();
                        } else if matches!(
                            operation,
                            Some(Operation::Connect | Operation::Check | Operation::Disconnect)
                        ) {
                            self.connection = ConnectionState::Error(error.clone());
                            self.last_check = Instant::now();
                        } else if self.settings.target == BackupTarget::Nas {
                            self.nas_check_pending = self.settings.nas.root.is_some();
                        } else if self.is_connected() {
                            // A failed transfer may have lost permission or connectivity after its initial check.
                            self.check_pending = true;
                        }
                    }
                    let failed = result.is_err() || self.save_error.is_some();
                    self.message = self
                        .save_error
                        .clone()
                        .unwrap_or_else(|| result.unwrap_or_else(|error| error));
                    if let Some(id) = self.active_notice.take() {
                        if let Some(notice) = self.notices.iter_mut().find(|n| n.id == id) {
                            notice.title = if failed {
                                "Sync needs attention"
                            } else {
                                "Sync complete"
                            }
                            .into();
                            notice.detail = self.message.clone();
                            notice.failed = failed;
                            if !failed {
                                notice.progress = 1.0;
                            }
                            notice.finished = Some(Instant::now());
                        }
                    }
                }
            }
        }
        self.notices.retain(|n| {
            n.finished
                .is_none_or(|t| t.elapsed() < Duration::from_secs(5))
        });
        if self.refresh_pending && !self.busy {
            self.refresh_pending = false;
            self.refresh();
        }
        if automatic_allowed
            && self.settings.automatic
            && self.ready()
            && !self.busy
            && self.last_auto.elapsed() > Duration::from_secs(120)
        {
            self.last_auto = Instant::now();
            if self
                .library
                .files
                .iter()
                .any(|p| self.settings.selected.contains(&p.path) && self.status(p) != "Backed up")
            {
                self.sync();
            }
        }
        if !self.busy
            && (self.nas_check_pending
                || (cloud_visible
                    && self.settings.nas.root.is_some()
                    && self.nas_last_check.elapsed() >= Duration::from_secs(300)))
        {
            self.check_nas();
        }
        if !self.busy
            && (self.check_pending
                || (cloud_visible
                    && self.settings.account.is_some()
                    && self.last_check.elapsed() >= Duration::from_secs(300)))
        {
            self.check_connection();
        }
    }
}
