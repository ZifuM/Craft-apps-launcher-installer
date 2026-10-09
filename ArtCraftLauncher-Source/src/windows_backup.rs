//! Shared backup status, with read-only Windows Cloud Files upload confirmation.
//! Other platforms report local copies without inventing provider upload state.
use super::{Cloud, CloudSettings, Receipt};
use crate::{Project, tr};
use std::{
    collections::HashMap,
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};
#[cfg(windows)]
use std::{
    fs::OpenOptions,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    sync::OnceLock,
};
#[cfg(windows)]
use windows_sys::Win32::{
    Storage::FileSystem::{
        FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FileAttributeTagInfo, GetFileInformationByHandleEx,
    },
    System::LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Local,
    Pending,
    Checking,
    Copied,
    Uploading,
    Confirmed,
    Changed,
    Partial,
    Unavailable,
    Error,
}
impl State {
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "Local only",
            Self::Pending => "Pending backup",
            Self::Checking => "Checking backup",
            Self::Copied => "Copied to sync folder",
            Self::Uploading => "Awaiting cloud upload",
            Self::Confirmed => "Backed up to cloud",
            Self::Changed => "Changes not backed up",
            Self::Partial => "Partially backed up",
            Self::Unavailable => "Backup unavailable",
            Self::Error => "Needs attention",
        }
    }
    pub fn has_copy(self) -> bool {
        matches!(self, Self::Copied | Self::Uploading | Self::Confirmed)
    }
}
pub struct Summary {
    pub state: State,
    pub date: Option<String>,
    pub tooltip: String,
}
impl Summary {
    pub fn date_label(&self) -> String {
        self.date
            .as_ref()
            .map(|date| tr(format!("Last backup: {date}")))
            .unwrap_or_else(|| tr("No backup recorded"))
    }
}
struct Observation {
    state: State,
    checked: Instant,
    bytes: u64,
}
pub struct Monitor {
    cache: HashMap<String, Observation>,
    tx: mpsc::Sender<(String, Observation)>,
    rx: mpsc::Receiver<(String, Observation)>,
    running: Option<std::thread::JoinHandle<()>>,
    last_poll: Option<Instant>,
}
impl Monitor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            cache: HashMap::new(),
            tx,
            rx,
            running: None,
            last_poll: None,
        }
    }
    pub fn tick(&mut self, settings: &CloudSettings) {
        while let Ok((id, observation)) = self.rx.try_recv() {
            self.cache.insert(id, observation);
        }
        if self.running.as_ref().is_some_and(|job| !job.is_finished()) {
            return;
        }
        self.running.take();
        let new_receipt = settings
            .receipts
            .iter()
            .any(|r| !self.cache.contains_key(&r.remote.id));
        if !new_receipt
            && self
                .last_poll
                .is_some_and(|time| time.elapsed() < Duration::from_secs(5))
        {
            return;
        }
        self.last_poll = Some(Instant::now());
        let active: std::collections::HashSet<_> =
            settings.receipts.iter().map(|r| &r.remote.id).collect();
        self.cache.retain(|id, _| active.contains(id));
        let receipts = settings.receipts.clone();
        if receipts.is_empty() {
            return;
        }
        let tx = self.tx.clone();
        self.running = Some(std::thread::spawn(move || {
            for receipt in receipts {
                let state = inspect(&receipt);
                let observation = Observation {
                    state,
                    checked: Instant::now(),
                    bytes: receipt.remote.bytes,
                };
                if tx.send((receipt.remote.id, observation)).is_err() {
                    break;
                }
            }
        }));
    }
    fn state(&self, receipt: &Receipt) -> State {
        self.cache
            .get(&receipt.remote.id)
            .filter(|o| {
                o.bytes == receipt.remote.bytes && o.checked.elapsed() < Duration::from_secs(20)
            })
            .map(|o| o.state)
            .unwrap_or(State::Checking)
    }
}
impl Cloud {
    /// Selection controls future backups, never whether an existing backup is displayed.
    pub fn backup_status(&self, project: &Project) -> Summary {
        let receipts: Vec<_> = self
            .settings
            .receipts
            .iter()
            .filter(|r| {
                r.path == project.path
                    && self
                        .settings
                        .folders
                        .get(&r.remote.provider)
                        .is_some_and(|f| f.path == r.remote.folder)
            })
            .collect();
        let mut details = Vec::new();
        let mut current = Vec::new();
        let mut latest = None;
        let mut has_old = false;
        let mut checking = false;
        let mut unavailable = false;
        for receipt in &receipts {
            let state = self.monitor.state(receipt);
            checking |= state == State::Checking;
            unavailable |= state == State::Unavailable;
            if state.has_copy() {
                if receipt.revision == project.revision {
                    current.push((receipt.remote.provider, state));
                } else {
                    has_old = true;
                }
            }
            if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&receipt.remote.modified) {
                if latest.is_none_or(|old| date > old) {
                    latest = Some(date);
                }
            }
            details.push(format!(
                "{}: {}{}",
                receipt.remote.provider.name(),
                tr(state.label()),
                if receipt.revision != project.revision {
                    format!(" · {}", tr("Changes not backed up"))
                } else {
                    String::new()
                }
            ));
        }
        let errors: Vec<_> = self
            .errors
            .iter()
            .filter(|((path, _), _)| path == &project.path)
            .collect();
        for ((_, provider), error) in &errors {
            details.push(format!("{}: {}", provider.name(), tr(*error)));
        }
        let all_current = !current.is_empty()
            && self
                .settings
                .targets
                .iter()
                .all(|provider| current.iter().any(|(p, _)| p == provider));
        let state = if !errors.is_empty() {
            State::Error
        } else if all_current {
            if current.iter().all(|(_, s)| *s == State::Confirmed) {
                State::Confirmed
            } else if current.iter().any(|(_, s)| *s == State::Uploading) {
                State::Uploading
            } else {
                State::Copied
            }
        } else if !current.is_empty() {
            State::Partial
        } else if checking {
            State::Checking
        } else if has_old {
            State::Changed
        } else if unavailable {
            State::Unavailable
        } else if self.settings.selected.contains(&project.path) {
            State::Pending
        } else {
            State::Local
        };
        let date = latest.map(|date| {
            date.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        });
        let mut summary = Summary {
            state,
            date,
            tooltip: String::new(),
        };
        summary.tooltip = format!("{}\n{}", tr(state.label()), summary.date_label());
        if !details.is_empty() {
            summary
                .tooltip
                .push_str(&format!("\n{}", details.join("\n")));
        }
        summary.tooltip.push_str(&format!("\n{}", tr("Backup dates show when the version was saved. Cloud confirmation comes from your sync app.")));
        if matches!(state, State::Copied | State::Partial) {
            summary.tooltip.push_str(&format!(
                "\n{}",
                tr("Upload unconfirmed. Check your provider's desktop app.")
            ));
        }
        summary
    }
}

// https://learn.microsoft.com/en-us/windows/win32/api/cfapi/ne-cfapi-cf_placeholder_state
// Dynamically load from System32 so unsupported Windows versions safely report unknown.
#[cfg(windows)]
type StateFn = unsafe extern "system" fn(u32, u32) -> u32;
#[cfg(windows)]
fn cloud_function() -> Option<StateFn> {
    static FUNCTION: OnceLock<Option<StateFn>> = OnceLock::new();
    *FUNCTION.get_or_init(|| unsafe {
        let name: Vec<u16> = "CldApi.dll\0".encode_utf16().collect();
        let module = LoadLibraryExW(
            name.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        );
        if module.is_null() {
            return None;
        }
        // Retain the module for the lifetime of the cached function pointer.
        GetProcAddress(
            module,
            c"CfGetPlaceholderStateFromAttributeTag".as_ptr().cast(),
        )
        .map(|function| {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, StateFn>(function)
        })
    })
}
#[cfg(windows)]
fn file_state(path: &Path, expected_bytes: Option<u64>) -> State {
    let Ok(file) = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
    else {
        return State::Unavailable;
    };
    let Ok(metadata) = file.metadata() else {
        return State::Unavailable;
    };
    if !metadata.is_file() || expected_bytes.is_some_and(|bytes| metadata.len() != bytes) {
        return State::Unavailable;
    }
    let Some(function) = cloud_function() else {
        return State::Copied;
    };
    let mut info = FILE_ATTRIBUTE_TAG_INFO {
        FileAttributes: 0,
        ReparseTag: 0,
    };
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    if success == 0 {
        return State::Copied;
    }
    let state = unsafe { function(info.FileAttributes, info.ReparseTag) };
    if state == u32::MAX || state & 1 == 0 {
        State::Copied
    } else if state & 8 != 0 {
        State::Confirmed
    } else {
        State::Uploading
    }
}
fn inspect(receipt: &Receipt) -> State {
    let path = Path::new(&receipt.remote.id);
    let file = file_state(path, Some(receipt.remote.bytes));
    let Some(parent) = path.parent() else {
        return State::Unavailable;
    };
    let manifest = file_state(&parent.join("backup.json"), None);
    if file == State::Unavailable || manifest == State::Unavailable {
        State::Unavailable
    } else if file == State::Confirmed && manifest == State::Confirmed {
        State::Confirmed
    } else if file == State::Uploading || manifest == State::Uploading {
        State::Uploading
    } else {
        State::Copied
    }
}

#[cfg(not(windows))]
fn file_state(path: &Path, expected_bytes: Option<u64>) -> State {
    // File-provider integration differs by service on Unix. Metadata can confirm
    // the local version only; do not read/hydrate contents or infer an upload.
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() && expected_bytes.is_none_or(|size| size == meta.len()) => {
            State::Copied
        }
        _ => State::Unavailable,
    }
}
