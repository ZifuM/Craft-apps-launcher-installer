use super::*;

#[derive(Clone)]
pub struct Download {
    pub name: String,
    pub url: String,
    pub bytes: u64,
    pub digest: Option<String>,
}
fn allowed(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "api.github.com"
                    | "raw.githubusercontent.com"
                    | "objects.githubusercontent.com"
                    | "release-assets.githubusercontent.com"
            )
        )
}
fn request(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let url = reqwest::Url::parse(url).map_err(|_| "The GitHub URL is invalid.")?;
    if !allowed(&url) {
        return Err("Use a public HTTPS GitHub plugin link.".into());
    }
    let client = Client::builder()
        .user_agent("ArtCraftMasterSuite-plugin-installer")
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 8 && allowed(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("Unsupported plugin download redirect")
            }
        }))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("GitHub could not be reached: {e}"))?;
    if response.status().as_u16() == 404 {
        return Err("No public release or file was found at this GitHub link. Use a published plugin release or an exact compiled file link.".into());
    }
    if matches!(response.status().as_u16(), 403 | 429) {
        return Err("GitHub is limiting requests or this download is private. Try later, or download the plugin in your browser and use Install from file.".into());
    }
    let response = response.error_for_status().map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("The plugin download exceeds the size limit.".into());
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("The plugin download exceeds the size limit.".into());
    }
    Ok(bytes)
}
pub(super) fn fetch(download: &Download) -> Result<Vec<u8>, String> {
    let bytes = request(&download.url, packages::PACKAGE_LIMIT)?;
    if let Some(expected) = download
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
    {
        if !hash(&bytes).eq_ignore_ascii_case(expected) {
            return Err(
                "The downloaded file does not match GitHub's checksum. Nothing was installed."
                    .into(),
            );
        }
    }
    Ok(bytes)
}
fn supported_asset(name: &str, app: &AppInfo) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".zip")
        || if app.id == "soundcraft" {
            name.ends_with(".clap") || name.ends_with(".vst3")
        } else {
            name.ends_with(".wasm")
        }
}
pub(super) fn resolve(input: &str, app: &AppInfo) -> Result<Vec<Download>, String> {
    let mut url = reqwest::Url::parse(input.trim())
        .map_err(|_| "Paste a GitHub repository, release or compiled plugin file link.")?;
    if !allowed(&url)
        || !matches!(
            url.host_str(),
            Some("github.com" | "raw.githubusercontent.com")
        )
    {
        return Err("Use a public github.com or raw.githubusercontent.com HTTPS link.".into());
    }
    url.set_fragment(None);
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    if parts.len() < 2 {
        return Err(
            "Paste a plugin repository link, including its owner and repository name.".into(),
        );
    }
    let name = parts.last().unwrap().to_string();
    if url.host_str() == Some("raw.githubusercontent.com")
        || (parts.len() >= 6 && parts[2..4] == ["releases", "download"])
    {
        return Ok(vec![Download {
            name,
            url: url.to_string(),
            bytes: 0,
            digest: None,
        }]);
    }
    if parts.len() >= 5 && matches!(parts[2], "blob" | "raw") {
        if !supported_asset(&name, app) {
            return Err(
                "Choose a compiled plugin file (.wasm, audio plugin or ZIP), not source code."
                    .into(),
            );
        }
        let mut raw = url.clone();
        raw.set_query(Some("raw=true"));
        return Ok(vec![Download {
            name,
            url: raw.to_string(),
            bytes: 0,
            digest: None,
        }]);
    }
    let suffix = if parts.len() == 2
        || (parts.len() == 3 && parts[2] == "releases")
        || (parts.len() == 4 && parts[2..4] == ["releases", "latest"])
    {
        "latest".to_owned()
    } else if parts.len() >= 5 && parts[2..4] == ["releases", "tag"] {
        format!("tags/{}", parts[4..].join("/"))
    } else {
        return Err("Use the plugin repository, its release page, or a compiled file link. Repository source folders cannot be installed.".into());
    };
    let repo = parts[1].trim_end_matches(".git");
    let json: Value = serde_json::from_slice(&request(
        &format!(
            "https://api.github.com/repos/{}/{repo}/releases/{suffix}",
            parts[0]
        ),
        4 * 1024 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for asset in json["assets"]
        .as_array()
        .ok_or("This release has no downloads.")?
    {
        let (Some(name), Some(url)) = (
            asset["name"].as_str(),
            asset["browser_download_url"].as_str(),
        ) else {
            continue;
        };
        if supported_asset(name, app) {
            result.push(Download {
                name: name.into(),
                url: url.into(),
                bytes: asset["size"].as_u64().unwrap_or(0),
                digest: asset["digest"].as_str().map(str::to_owned),
            });
        }
    }
    if result.is_empty() {
        return Err(format!(
            "This release has no compiled {} plugin downloads. Source archives and app installers are not plugins.",
            app.name
        ));
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}
