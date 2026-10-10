//! Google Drive desktop OAuth and versioned uploads. Tokens never enter preferences or logs.
use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{StatusCode, blocking::Response};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
};

pub const CLIENT_ID: &str =
    "500269938170-gn17tokb63uivgok2dtcu10n7tg335ei.apps.googleusercontent.com";
const API: &str = "https://www.googleapis.com/drive/v3";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const CHUNK: usize = 8 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub label: String,
}
#[derive(Serialize, Deserialize)]
struct Credential {
    client_id: String,
    refresh_token: String,
    account: Account,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub name: String,
    pub bytes: u64,
    pub modified: String,
    pub digest: String,
    pub source: String,
    pub revision: u64,
    pub app: String,
    #[serde(default)]
    pub destination: String,
    #[serde(default)]
    pub local_file: Option<String>,
}

pub fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Sync cancelled.".into())
    } else {
        Ok(())
    }
}
fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not prepare the cloud connection.".into())
}
fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("ArtCraft Master Suite Google Drive", CLIENT_ID).map_err(|_| {
        "The system credential store is unavailable. Unlock your keychain and try again.".into()
    })
}
fn store(credential: &Credential) -> Result<(), String> {
    let value =
        serde_json::to_string(credential).map_err(|_| "Could not save the cloud connection.")?;
    entry()?.set_password(&value).map_err(|_| "Could not save the connection in the system credential store. Unlock your keychain and retry.".into())
}
pub fn disconnect() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("Could not remove the connection from the system credential store.".into()),
    }
}
fn send(request: reqwest::blocking::RequestBuilder) -> Result<Response, String> {
    request
        .send()
        .map_err(|_| "Could not reach Google Drive. Check your connection and retry.".into())
}
fn read_json(response: Response) -> Result<Value, String> {
    let status = response.status();
    let mut data = Vec::new();
    response
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|_| "Google's response was interrupted.")?;
    if data.len() > 4 * 1024 * 1024 {
        return Err("Google's response was too large.".into());
    }
    let value: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
    if !status.is_success() {
        let oauth = value["error"].as_str().unwrap_or_default();
        let description = value["error_description"].as_str().unwrap_or_default();
        let reason = value["error"]["errors"][0]["reason"]
            .as_str()
            .unwrap_or_default();
        return Err(if description.contains("client_secret") {
            "Google needs your Desktop app credentials. In Google Auth Platform, open Clients, select this Desktop app and download its JSON. Add it as google-desktop-client.json in the Master Suite cloud settings folder, then retry.".into()
        } else if oauth == "invalid_client" || oauth == "unauthorized_client" {
            "Google rejected this app's credentials. Check that the client is a Desktop app in the correct project and that its downloaded JSON matches this client ID.".into()
        } else if oauth == "invalid_grant" || status == StatusCode::UNAUTHORIZED {
            "Your Google connection expired or was revoked. Reconnect Google Drive.".into()
        } else if reason == "storageQuotaExceeded" {
            "Google Drive is full. Free up space and retry.".into()
        } else if status == StatusCode::FORBIDDEN {
            "Google denied access. Check that the Drive API is enabled and your account is an allowed test user, then reconnect.".into()
        } else if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            "Google Drive is temporarily busy. Please retry shortly.".into()
        } else {
            format!(
                "Google Drive could not complete the request (HTTP {}).",
                status.as_u16()
            )
        });
    }
    if value.is_null() {
        return Err("Google returned an unreadable response.".into());
    }
    Ok(value)
}
fn random() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "Could not securely start sign-in.")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
// Desktop clients may have an optional public client secret in Google's downloaded JSON.
// It is kept outside the repository; never substitute a web/server client credential.
fn client_secret() -> Result<Option<String>, String> {
    if let Some(value) =
        option_env!("ARTCRAFT_GOOGLE_DESKTOP_CLIENT_SECRET").filter(|s| !s.is_empty())
    {
        return Ok(Some(value.to_owned()));
    }
    let Some(root) = app_data() else {
        return Ok(None);
    };
    let path = root.join("cloud/google-desktop-client.json");
    if !path.exists() {
        return Ok(None);
    }
    let file = fs::File::open(path).map_err(|_| "Could not read Google desktop credentials.")?;
    let value: Value = serde_json::from_reader(file.take(65536))
        .map_err(|_| "Google desktop credentials are invalid.")?;
    let installed = &value["installed"];
    if installed["client_id"].as_str() != Some(CLIENT_ID) {
        return Err("Google desktop credentials belong to a different client ID.".into());
    }
    Ok(installed["client_secret"].as_str().map(str::to_owned))
}
fn token(client: &Client, mut form: Vec<(&str, String)>) -> Result<Value, String> {
    form.push(("client_id", CLIENT_ID.into()));
    if let Some(secret) = client_secret()? {
        form.push(("client_secret", secret));
    }
    let tokens = read_json(send(
        client
            .post("https://oauth2.googleapis.com/token")
            .form(&form),
    )?)?;
    if tokens["scope"]
        .as_str()
        .is_some_and(|scope| !scope.split_whitespace().any(|s| s == SCOPE))
    {
        return Err("Google Drive permission was not granted. Reconnect and approve access to files used by Master Suite.".into());
    }
    Ok(tokens)
}
fn read_account(http: &Client, access: &str) -> Result<Account, String> {
    let about = read_json(send(
        http.get(format!("{API}/about"))
            .bearer_auth(access)
            .query(&[("fields", "user(displayName,emailAddress,permissionId)")]),
    )?)?;
    let user = &about["user"];
    Ok(Account {
        id: user["permissionId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("Google did not identify the account.")?
            .into(),
        label: user["emailAddress"]
            .as_str()
            .or(user["displayName"].as_str())
            .unwrap_or("Google Drive")
            .into(),
    })
}
pub fn sign_in(cancel: &AtomicBool) -> Result<Account, String> {
    // Report mismatched local credentials before asking the user to sign in.
    let _ = client_secret()?;
    let verifier = random()?;
    let state = random()?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|_| "Could not open the local sign-in callback.")?;
    listener
        .set_nonblocking(true)
        .map_err(|_| "Could not prepare sign-in.")?;
    let address = listener
        .local_addr()
        .map_err(|_| "Could not prepare sign-in.")?;
    let redirect = format!("http://127.0.0.1:{}/", address.port());
    let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("client_id", CLIENT_ID),
        ("redirect_uri", redirect.as_str()),
        ("response_type", "code"),
        ("scope", SCOPE),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("state", state.as_str()),
        ("access_type", "offline"),
        ("prompt", "consent select_account"),
    ]);
    open_url(url.as_str());
    let started = Instant::now();
    let code = loop {
        cancelled(cancel)?;
        if started.elapsed() > Duration::from_secs(300) {
            return Err("Google sign-in timed out. Click Connect to try again.".into());
        }
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                let mut line = String::new();
                if BufReader::new((&mut stream).take(8192))
                    .read_line(&mut line)
                    .is_err()
                {
                    continue;
                }
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() != 3 || parts[0] != "GET" || !parts[1].starts_with("/?") {
                    continue;
                }
                let Ok(callback) = reqwest::Url::parse(&format!("{redirect}{}", &parts[1][1..]))
                else {
                    continue;
                };
                let query: HashMap<_, _> = callback.query_pairs().into_owned().collect();
                if query.get("state") != Some(&state) {
                    continue;
                }
                let denied = query.contains_key("error");
                let body = if denied {
                    "Sign-in was not completed. You can return to ArtCraft Master Suite."
                } else {
                    "You can return to ArtCraft Master Suite. It is finishing your Google Drive connection."
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                if denied {
                    return Err("Google sign-in was cancelled or denied.".into());
                }
                if let Some(code) = query.get("code").filter(|v| !v.is_empty()) {
                    break code.clone();
                }
                return Err("Google did not return a sign-in code.".into());
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100))
            }
            Err(_) => return Err("The local sign-in callback failed. Please reconnect.".into()),
        }
    };
    cancelled(cancel)?;
    let http = client()?;
    let tokens = token(
        &http,
        vec![
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect),
            ("grant_type", "authorization_code".into()),
        ],
    )?;
    let access = tokens["access_token"]
        .as_str()
        .ok_or("Google did not return an access token.")?;
    let refresh = tokens["refresh_token"]
        .as_str()
        .ok_or("Google did not grant offline access. Reconnect and approve access.")?;
    let account = read_account(&http, access)?;
    cancelled(cancel)?;
    store(&Credential {
        client_id: CLIENT_ID.into(),
        refresh_token: refresh.into(),
        account: account.clone(),
    })?;
    Ok(account)
}

pub struct Drive {
    http: Client,
    access: String,
    account: String,
}
impl Drive {
    pub fn connect(account: &Account) -> Result<Self, String> {
        let raw = entry()?.get_password().map_err(|_| "Your Google connection is unavailable. Unlock your keychain or reconnect Google Drive.")?;
        let mut credential: Credential = serde_json::from_str(&raw)
            .map_err(|_| "Reconnect Google Drive to renew your connection.")?;
        if credential.client_id != CLIENT_ID || credential.account.id != account.id {
            return Err("Reconnect the Google account selected for these backups.".into());
        }
        let http = client()?;
        let tokens = token(
            &http,
            vec![
                ("grant_type", "refresh_token".into()),
                ("refresh_token", credential.refresh_token.clone()),
            ],
        )?;
        if let Some(refresh) = tokens["refresh_token"].as_str() {
            credential.refresh_token = refresh.into();
            store(&credential)?;
        }
        let access = tokens["access_token"]
            .as_str()
            .ok_or("Google did not return an access token.")?;
        // A cached account or token response alone is not proof of Drive access.
        let verified = read_account(&http, access)?;
        if verified.id != account.id {
            return Err("Google connected a different account. Reconnect the account selected for these backups.".into());
        }
        Ok(Self {
            http,
            access: access.into(),
            account: account.id.clone(),
        })
    }
    fn get(&self, id: &str) -> Result<Value, String> {
        read_json(send(
            self.http
                .get(format!("{API}/files/{}", component(id)))
                .bearer_auth(&self.access)
                .query(&[(
                    "fields",
                    "id,name,size,modifiedTime,sha256Checksum,appProperties,parents",
                )]),
        )?)
    }
    fn folder(&self) -> Result<String, String> {
        let result = read_json(send(self.http.get(format!("{API}/files")).bearer_auth(&self.access)
            .query(&[("q", "trashed = false and mimeType = 'application/vnd.google-apps.folder' and appProperties has { key='artcraft' and value='root-v1' }"),
                ("fields", "files(id)"), ("pageSize", "1")]))?)?;
        if let Some(id) = result["files"][0]["id"].as_str() {
            return Ok(id.into());
        }
        let result = read_json(send(self.http.post(format!("{API}/files")).bearer_auth(&self.access)
            .json(&json!({"name":"ArtCraft Master Suite", "mimeType":"application/vnd.google-apps.folder", "appProperties":{"artcraft":"root-v1"}})))?)?;
        Ok(result["id"]
            .as_str()
            .ok_or("Google did not create the backup folder.")?
            .into())
    }
    fn destination_folder(
        &self,
        destination: &cloud_layout::Destination,
        cancel: &AtomicBool,
    ) -> Result<String, String> {
        let mut parent = self.folder()?;
        for name in &destination.folders {
            cancelled(cancel)?;
            let escape = |value: &str| value.replace('\\', "\\\\").replace('\'', "\\'");
            let query = format!(
                "trashed = false and mimeType = 'application/vnd.google-apps.folder' and '{}' in parents and name = '{}'",
                escape(&parent),
                escape(name)
            );
            let result = read_json(send(
                self.http
                    .get(format!("{API}/files"))
                    .bearer_auth(&self.access)
                    .query(&[
                        ("q", query.as_str()),
                        ("fields", "files(id)"),
                        ("pageSize", "1"),
                    ]),
            )?)?;
            parent = if let Some(id) = result["files"][0]["id"].as_str() {
                id.into()
            } else {
                let created = read_json(send(self.http.post(format!("{API}/files")).bearer_auth(&self.access)
                    .json(&json!({"name":name, "mimeType":"application/vnd.google-apps.folder", "parents":[parent],
                        "appProperties":{"artcraft":"workspace-folder-v1"}})))?)?;
                created["id"]
                    .as_str()
                    .ok_or("Google did not create the workspace folder.")?
                    .into()
            };
        }
        Ok(parent)
    }
    fn confirm_upload(
        &self,
        id: &str,
        snapshot: &Snapshot,
        destination: &cloud_layout::Destination,
        folder: &str,
    ) -> Result<Version, String> {
        let file = self.get(id)?;
        let saved = version(&file)?;
        if saved.bytes != snapshot.size
            || saved.digest != snapshot.digest
            || saved.destination != destination.key()
            || !file["parents"]
                .as_array()
                .is_some_and(|parents| parents.iter().any(|p| p.as_str() == Some(folder)))
        {
            return Err(
                "Google could not confirm the complete file in its workspace folder. Retry Sync."
                    .into(),
            );
        }
        Ok(saved)
    }
    pub fn versions(&self, cancel: &AtomicBool) -> Result<Vec<Version>, String> {
        let mut files = Vec::new();
        let mut page = String::new();
        loop {
            cancelled(cancel)?;
            let mut request = self.http.get(format!("{API}/files")).bearer_auth(&self.access).query(&[
                ("q", "trashed = false and appProperties has { key='artcraft' and value='project-v1' }"),
                ("fields", "nextPageToken,files(id,name,size,modifiedTime,sha256Checksum,appProperties)"), ("pageSize", "1000"),
            ]);
            if !page.is_empty() {
                request = request.query(&[("pageToken", &page)]);
            }
            let value = read_json(send(request)?)?;
            for file in value["files"]
                .as_array()
                .ok_or("Google returned an invalid file list.")?
            {
                files.push(version(file)?);
            }
            if files.len() > 100000 {
                return Err("The cloud library exceeds 100,000 versions.".into());
            }
            page = value["nextPageToken"].as_str().unwrap_or_default().into();
            if page.is_empty() {
                break;
            }
        }
        files.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(files)
    }
    pub fn upload(
        &self,
        project: &Project,
        destination: &cloud_layout::Destination,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64, u64, &str),
    ) -> Result<Version, String> {
        cancelled(cancel)?;
        progress(0, project.size_bytes, "Preparing file");
        let snapshot = Snapshot::create(project, cancel)?;
        progress(0, snapshot.size, "Preparing workspace folders");
        let folder = self.destination_folder(destination, cancel)?;
        // Reuse a confirmed identical version; a retry must not duplicate an upload that already committed.
        let existing = read_json(send(self.http.get(format!("{API}/files")).bearer_auth(&self.access).query(&[
            ("q", format!("trashed = false and appProperties has {{ key='artcraft' and value='project-v1' }} and appProperties has {{ key='source' and value='{}' }} and appProperties has {{ key='sha256' and value='{}' }}", snapshot.source, snapshot.digest)),
            ("fields", "files(id,name,size,modifiedTime,sha256Checksum,appProperties,parents)".into()), ("pageSize", "1".into())
        ]))?)?;
        if let Some(file) = existing["files"].as_array().and_then(|v| v.first()) {
            let saved = version(file)?;
            if saved.bytes == snapshot.size && saved.digest == snapshot.digest {
                cancelled(cancel)?;
                // Older flat uploads are moved on the next sync, without copying their bytes again.
                let parents: Vec<_> = file["parents"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                let mut properties = file["appProperties"].clone();
                properties["destination"] = json!(destination.key());
                properties["revision"] = json!(project.revision.to_string());
                properties["app"] = json!(project.app.id);
                let mut request = self
                    .http
                    .patch(format!("{API}/files/{}", component(&saved.id)))
                    .bearer_auth(&self.access)
                    .json(&json!({"appProperties":properties}));
                if !parents.contains(&folder.as_str()) {
                    request = request.query(&[("addParents", &folder)]);
                }
                let previous = parents
                    .into_iter()
                    .filter(|parent| *parent != folder)
                    .collect::<Vec<_>>()
                    .join(",");
                if !previous.is_empty() {
                    request = request.query(&[("removeParents", previous)]);
                }
                read_json(send(request)?)?;
                let saved = self.confirm_upload(&saved.id, &snapshot, destination, &folder)?;
                progress(snapshot.size, snapshot.size, "Already uploaded");
                return Ok(saved);
            }
        }
        let generated = read_json(send(
            self.http
                .get(format!("{API}/files/generateIds"))
                .bearer_auth(&self.access)
                .query(&[("count", "1"), ("space", "drive"), ("type", "files")]),
        )?)?;
        let id = generated["ids"][0]
            .as_str()
            .ok_or("Google did not reserve an upload ID.")?;
        let metadata = json!({"id":id, "name":project.path.file_name().unwrap_or_default().to_string_lossy(), "parents":[folder],
            "appProperties":{"artcraft":"project-v1", "source":snapshot.source, "sha256":snapshot.digest,
                "revision":project.revision.to_string(), "app":project.app.id, "account":self.account,
                "destination":destination.key()}});
        let response = send(
            self.http
                .post("https://www.googleapis.com/upload/drive/v3/files")
                .bearer_auth(&self.access)
                .query(&[("uploadType", "resumable")])
                .header("X-Upload-Content-Type", "application/octet-stream")
                .header("X-Upload-Content-Length", snapshot.size)
                .json(&metadata),
        )?;
        if !response.status().is_success() {
            return read_json(response).and_then(|_| Err("Could not start the upload.".into()));
        }
        let session = response
            .headers()
            .get("Location")
            .and_then(|v| v.to_str().ok())
            .ok_or("Google did not provide an upload session.")?;
        let session = reqwest::Url::parse(session).map_err(|_| "Invalid Google upload session.")?;
        if session.scheme() != "https" || session.host_str() != Some("www.googleapis.com") {
            return Err("Google returned an unexpected upload address.".into());
        }
        let mut file = fs::File::open(&snapshot.path).map_err(|e| e.to_string())?;
        let mut offset = 0;
        let mut failures = 0;
        loop {
            cancelled(cancel)?;
            let len = (snapshot.size - offset).min(CHUNK as u64) as usize;
            let mut bytes = vec![0; len];
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
            file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
            let range = if snapshot.size == 0 {
                "bytes */0".to_owned()
            } else {
                format!(
                    "bytes {}-{}/{}",
                    offset,
                    offset + len as u64 - 1,
                    snapshot.size
                )
            };
            let result = send(
                self.http
                    .put(session.clone())
                    .bearer_auth(&self.access)
                    .header("Content-Type", "application/octet-stream")
                    .header("Content-Range", range)
                    .body(bytes),
            );
            let response = match result {
                Ok(response)
                    if !response.status().is_server_error()
                        && response.status() != StatusCode::TOO_MANY_REQUESTS =>
                {
                    response
                }
                _ => {
                    failures += 1;
                    if failures > 3 {
                        return Err("Upload interrupted. Retry Sync to continue safely.".into());
                    }
                    for _ in 0..failures * 10 {
                        cancelled(cancel)?;
                        thread::sleep(Duration::from_millis(100));
                    }
                    send(
                        self.http
                            .put(session.clone())
                            .bearer_auth(&self.access)
                            .header("Content-Length", 0)
                            .header("Content-Range", format!("bytes */{}", snapshot.size)),
                    )?
                }
            };
            if response.status() == StatusCode::PERMANENT_REDIRECT {
                let next = response
                    .headers()
                    .get("Range")
                    .and_then(|r| r.to_str().ok())
                    .and_then(|r| r.strip_prefix("bytes=0-"))
                    .and_then(|r| r.parse::<u64>().ok())
                    .and_then(|n| n.checked_add(1))
                    .unwrap_or(0);
                if next > snapshot.size || next < offset {
                    return Err("Google returned an invalid upload position.".into());
                }
                if next == offset {
                    failures += 1;
                    if failures > 3 {
                        return Err("Upload stalled. Please retry Sync.".into());
                    }
                } else {
                    failures = 0;
                }
                offset = next;
                progress(offset, snapshot.size, "Uploading to Google Drive");
                if snapshot.size != 0 && offset == snapshot.size {
                    break;
                }
                continue;
            }
            if !response.status().is_success() {
                // The final reply can be lost after Google commits the file.
                if let Ok(saved) = self.confirm_upload(id, &snapshot, destination, &folder) {
                    return Ok(saved);
                }
                read_json(response)?;
            }
            break;
        }
        progress(snapshot.size, snapshot.size, "Confirming upload");
        self.confirm_upload(id, &snapshot, destination, &folder)
    }
    pub fn restore(
        &self,
        saved: &Version,
        destination: &Path,
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let current = version(&self.get(&saved.id)?)?;
        if current.digest != saved.digest || current.bytes != saved.bytes {
            return Err(
                "This cloud version has changed. Refresh the library before restoring.".into(),
            );
        }
        let mut response = send(
            self.http
                .get(format!("{API}/files/{}", component(&saved.id)))
                .bearer_auth(&self.access)
                .query(&[("alt", "media")]),
        )?;
        if !response.status().is_success() {
            read_json(response)?;
            return Err("Could not download the saved version.".into());
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|e| format!("Choose a new filename; existing files are kept: {e}"))?;
        let result: Result<(), String> = (|| {
            let mut hash = Sha256::new();
            let mut count = 0;
            let mut buffer = vec![0; 1024 * 1024];
            loop {
                cancelled(cancel)?;
                let n = response
                    .read(&mut buffer)
                    .map_err(|_| "Download interrupted. Please retry.")?;
                if n == 0 {
                    break;
                }
                count += n as u64;
                if count > saved.bytes {
                    return Err("Download size did not match the saved version.".into());
                }
                hash.update(&buffer[..n]);
                output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            }
            if count != saved.bytes || format!("{:x}", hash.finalize()) != saved.digest {
                return Err("Download integrity check failed.".into());
            }
            output.sync_all().map_err(|e| e.to_string())
        })();
        drop(output);
        if result.is_err() {
            let _ = fs::remove_file(destination);
        }
        result
    }
}
fn component(value: &str) -> String {
    reqwest::Url::parse("https://www.googleapis.com/")
        .map(|mut u| {
            u.path_segments_mut().unwrap().push(value);
            u.path()[1..].to_owned()
        })
        .unwrap()
}
pub fn source_id(path: &Path) -> String {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    format!("{:x}", Sha256::digest(path.to_string_lossy().as_bytes()))
}
fn version(file: &Value) -> Result<Version, String> {
    let digest = file["sha256Checksum"]
        .as_str()
        .ok_or("Google has not yet confirmed this file's checksum. Refresh and retry.")?;
    if digest.len() != 64 || !digest.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid cloud file checksum.".into());
    }
    let props = &file["appProperties"];
    Ok(Version {
        id: file["id"]
            .as_str()
            .ok_or("Cloud file ID is missing.")?
            .into(),
        name: file["name"]
            .as_str()
            .ok_or("Cloud filename is missing.")?
            .into(),
        bytes: file["size"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or("Cloud file size is missing.")?,
        modified: file["modifiedTime"].as_str().unwrap_or_default().into(),
        digest: digest.into(),
        source: props["source"].as_str().unwrap_or_default().into(),
        revision: props["revision"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        app: props["app"].as_str().unwrap_or_default().into(),
        destination: props["destination"].as_str().unwrap_or_default().into(),
        local_file: None,
    })
}
pub(crate) struct Snapshot {
    pub path: PathBuf,
    source: String,
    pub size: u64,
    pub digest: String,
    created: bool,
}
impl Drop for Snapshot {
    fn drop(&mut self) {
        if self.created {
            let _ = fs::remove_file(&self.path);
        }
    }
}
impl Snapshot {
    pub fn create(project: &Project, cancel: &AtomicBool) -> Result<Self, String> {
        let dir = app_data()
            .ok_or("App data folder unavailable.")?
            .join("cloud/staging");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut snapshot = Self {
            path: dir.join(format!("{}.upload", random()?)),
            source: source_id(&project.path),
            size: 0,
            digest: String::new(),
            created: false,
        };
        let mut input = fs::File::open(&project.path).map_err(|e| e.to_string())?;
        if project_revision(&project.path) != project.revision {
            return Err("The project changed. Refresh the library and retry Sync.".into());
        }
        let expected = input.metadata().map_err(|e| e.to_string())?.len();
        let mut output = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&snapshot.path)
            .map_err(|e| e.to_string())?;
        snapshot.created = true;
        let result: Result<(), String> = (|| {
            let mut hash = Sha256::new();
            let mut buffer = vec![0; 1024 * 1024];
            loop {
                cancelled(cancel)?;
                let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                hash.update(&buffer[..n]);
                snapshot.size += n as u64;
            }
            if snapshot.size != expected || project_revision(&project.path) != project.revision {
                return Err(
                    "The project changed while preparing the upload. Save it and retry.".into(),
                );
            }
            output.sync_all().map_err(|e| e.to_string())?;
            snapshot.digest = format!("{:x}", hash.finalize());
            Ok(())
        })();
        drop(output);
        result?;
        Ok(snapshot)
    }
}
