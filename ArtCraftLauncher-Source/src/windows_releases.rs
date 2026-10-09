//! Public release discovery for every desktop platform. No user credentials are read or bundled.
//! The REST API is preferred; public release pages remain usable when its shared
//! unauthenticated quota is exhausted. All downloads still require checksums.
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const TTL: u64 = 300;
const MAX_METADATA: u64 = 4 * 1024 * 1024;
static DISCOVERY: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize)]
struct Cached {
    at: u64,
    value: Value,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn cache_path(key: &str) -> Option<std::path::PathBuf> {
    crate::platform::data_dir().map(|p| {
        p.join("release-cache")
            .join(format!("{}.json", key.replace(['/', '?', '=', '&'], "_")))
    })
}
fn read_cache(key: &str) -> Option<Value> {
    let path = cache_path(key)?;
    if fs::metadata(&path).ok()?.len() > MAX_METADATA {
        return None;
    }
    let saved: Cached = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    (saved.at <= now() && now() - saved.at < TTL).then_some(saved.value)
}
fn save_cache(key: &str, value: &Value) {
    if let Some(path) = cache_path(key) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec(&Cached {
            at: now(),
            value: value.clone(),
        }) {
            let _ = fs::write(path, bytes);
        }
    }
}
fn limited_text(response: reqwest::blocking::Response) -> Result<String, String> {
    let response = response
        .error_for_status()
        .map_err(|e| format!("GitHub release service is unavailable: {e}"))?;
    let mut text = String::new();
    response
        .take(MAX_METADATA + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() as u64 > MAX_METADATA {
        return Err("Release metadata exceeds the size limit".into());
    }
    Ok(text)
}
fn api(client: &Client, endpoint: &str) -> Result<Value, String> {
    let retry_path = cache_path("api-retry-after");
    let retry = retry_path
        .as_ref()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    if retry > now() {
        return Err("GitHub API is cooling down after its rate limit".into());
    }
    let response = client
        .get(format!("https://api.github.com/repos/{endpoint}"))
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .send()
        .map_err(|e| e.to_string())?;
    if matches!(response.status().as_u16(), 403 | 429) {
        let reset = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        let delay = response
            .headers()
            .get("retry-after")
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        let until = reset
            .unwrap_or(now() + delay)
            .max(now() + 60)
            .min(now() + 86400);
        if let Some(path) = retry_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, until.to_string());
        }
        return Err("GitHub API rate limit reached".into());
    }
    serde_json::from_str(&limited_text(response)?)
        .map_err(|e| format!("Invalid GitHub release metadata: {e}"))
}
pub fn version(text: &str) -> Option<semver::Version> {
    let text = text.trim().trim_start_matches('v');
    let core = text.split(['-', '+']).next()?;
    let normalized = if core.split('.').count() == 2 {
        format!("{}.0{}", core, &text[core.len()..])
    } else {
        text.to_owned()
    };
    semver::Version::parse(&normalized).ok()
}
fn safe_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() < 200
        && tag
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-+".contains(&c))
}
fn latest_tag(client: &Client, repo: &str) -> Result<String, String> {
    let response = client
        .get(format!("https://github.com/{repo}/releases/latest"))
        .timeout(Duration::from_secs(25))
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let url = response.url();
    let prefix = format!("/{repo}/releases/tag/");
    let tag = url
        .path()
        .strip_prefix(&prefix)
        .filter(|tag| safe_tag(tag))
        .ok_or("No published stable release was found")?;
    if url.scheme() != "https" || url.host_str() != Some("github.com") {
        return Err("Unexpected release host".into());
    }
    Ok(tag.to_owned())
}
fn assets(client: &Client, repo: &str, tag: &str) -> Result<Vec<Value>, String> {
    if !safe_tag(tag) {
        return Err("Invalid release tag".into());
    }
    let html = limited_text(
        client
            .get(format!(
                "https://github.com/{repo}/releases/expanded_assets/{tag}"
            ))
            .timeout(Duration::from_secs(25))
            .send()
            .map_err(|e| e.to_string())?,
    )?;
    let prefix = format!("/{repo}/releases/download/{tag}/");
    let mut result = Vec::new();
    // Only accept actual asset hrefs within the exact official repo and tag.
    // Source archives and unrelated links cannot become install candidates.
    for chunk in html.split("href=\"").skip(1) {
        let Some(href) = chunk.split('"').next() else {
            continue;
        };
        let relative = href.strip_prefix("https://github.com").unwrap_or(href);
        let Some(name) = relative.strip_prefix(&prefix) else {
            continue;
        };
        if name.is_empty()
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-+".contains(&c))
        {
            continue;
        }
        if result
            .iter()
            .any(|a: &Value| a["name"].as_str() == Some(name))
        {
            continue;
        }
        result.push(json!({"name":name,"browser_download_url":format!("https://github.com{relative}"),"size":0,"digest":null}));
    }
    Ok(result)
}
pub fn latest_app(client: &Client, slug: &str) -> Result<Value, String> {
    let _guard = DISCOVERY.lock().map_err(|_| "Release discovery is busy")?;
    let repo = format!("storytold/{slug}");
    let key = format!("{repo}-stable");
    if let Some(value) = read_cache(&key) {
        return Ok(value);
    }
    let value = match api(client, &format!("{repo}/releases/latest")) {
        Ok(value) => value,
        Err(_) => {
            let tag=latest_tag(client,&repo).map_err(|e|format!("Could not check official releases. Check your internet connection and try again. {e}"))?;
            let found = assets(client, &repo, &tag)?;
            if found.is_empty() {
                return Err("The official release does not list any downloadable files".into());
            }
            json!({"tag_name":tag,"draft":false,"prerelease":false,"assets":found})
        }
    };
    save_cache(&key, &value);
    Ok(value)
}
pub fn suite(
    client: &Client,
    repo: &str,
    current: &str,
    include_prerelease: bool,
) -> Result<Value, String> {
    let _guard = DISCOVERY.lock().map_err(|_| "Release discovery is busy")?;
    let key = format!(
        "{repo}-{}-{current}",
        if include_prerelease { "all" } else { "stable" }
    );
    if let Some(value) = read_cache(&key) {
        return Ok(value);
    }
    if let Ok(value) = api(client, &format!("{repo}/releases?per_page=100")) {
        if value.is_array() {
            save_cache(&key, &value);
            return Ok(value);
        }
    }
    let installed = version(current).ok_or("Installed version is invalid")?;
    let mut tags = Vec::new();
    if include_prerelease {
        let xml = limited_text(
            client
                .get(format!("https://github.com/{repo}/releases.atom"))
                .timeout(Duration::from_secs(25))
                .send()
                .map_err(|e| e.to_string())?,
        )?;
        let feed = roxmltree::Document::parse(&xml)
            .map_err(|e| format!("Could not read the public release feed: {e}"))?;
        let prefix = format!("https://github.com/{repo}/releases/tag/");
        for link in feed
            .descendants()
            .filter(|n| n.has_tag_name("link") && n.attribute("rel") == Some("alternate"))
        {
            if let Some(tag) = link
                .attribute("href")
                .and_then(|url| url.strip_prefix(&prefix))
                .filter(|tag| safe_tag(tag))
            {
                if version(tag).is_some_and(|v| v.cmp_precedence(&installed).is_gt()) && !tags.iter().any(|t| t == tag) {
                    tags.push(tag.to_owned());
                }
            }
        }
    } else {
        let tag = latest_tag(client, repo)?;
        if version(&tag).is_some_and(|v| v.cmp_precedence(&installed).is_gt()) {
            tags.push(tag);
        }
    }
    tags.sort_by(|a,b| version(b).unwrap().cmp_precedence(&version(a).unwrap()));
    let mut releases = Vec::new();
    for tag in tags.into_iter().take(10) {
        let found = assets(client, repo, &tag)?;
        releases.push(
            json!({"tag_name":tag,"draft":false,"prerelease":include_prerelease,"assets":found}),
        );
    }
    let value = Value::Array(releases);
    save_cache(&key, &value);
    Ok(value)
}
pub fn download_size(client: &Client, url: &str) -> Result<u64, String> {
    let response = client
        .head(url)
        .timeout(Duration::from_secs(25))
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not read the installer size: {e}"))?;
    response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| {
            "The release server did not provide the installer size. Try again later.".into()
        })
}
