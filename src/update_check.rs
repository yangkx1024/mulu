use semver::Version;
use serde::Deserialize;

const API_URL: &str = "https://api.github.com/repos/yangkx1024/mulu/releases/latest";
const USER_AGENT: &str = concat!("mulu/", env!("CARGO_PKG_VERSION"));

pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
    html_url: String,
}

pub async fn check_for_update() -> Option<UpdateInfo> {
    tokio::task::spawn_blocking(fetch_latest)
        .await
        .ok()
        .flatten()
}

fn fetch_latest() -> Option<UpdateInfo> {
    let current = Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    let release: LatestRelease = ureq::get(API_URL)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(8))
        .call()
        .ok()?
        .into_json()
        .ok()?;
    let version_str = release.tag_name.trim().trim_start_matches('v');
    let latest = Version::parse(version_str).ok()?;
    if latest <= current {
        return None;
    }
    Some(UpdateInfo {
        version: version_str.to_string(),
        url: release.html_url,
    })
}
