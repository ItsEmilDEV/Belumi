use serde::Deserialize;
use std::cmp::Ordering;

pub const RELEASES_URL: &str = "https://api.github.com/repos/ItsEmilDEV/Belumi/releases?per_page=15";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Nightly,
}

impl Channel {
    pub fn parse(s: &str) -> Self {
        if s.trim().eq_ignore_ascii_case("nightly") {
            Channel::Nightly
        } else {
            Channel::Stable
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Nightly => "nightly",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Release {
    #[serde(rename = "tag_name")]
    pub tag: String,
    #[serde(rename = "html_url")]
    pub url: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
}

pub fn parse_releases(json: &str) -> Vec<Release> {
    serde_json::from_str(json).unwrap_or_default()
}

#[derive(Debug, PartialEq, Eq)]
struct Version {
    core: [u64; 3],
    pre: Option<String>,
}

fn parse_version(s: &str) -> Option<Version> {
    let s = s.trim().trim_start_matches('v');
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p.to_owned())),
        None => (s, None),
    };
    let mut parts = core.split('.');
    let mut out = [0u64; 3];
    for slot in &mut out {
        *slot = parts.next()?.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(Version { core: out, pre })
}

fn compare_pre(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.split('.'), b.split('.'));
    loop {
        match (x.next(), y.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) => {
                let o = match (p.parse::<u64>(), q.parse::<u64>()) {
                    (Ok(m), Ok(n)) => m.cmp(&n),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => p.cmp(q),
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
        }
    }
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (Some(c), Some(n)) = (parse_version(candidate), parse_version(current)) else {
        return false;
    };
    match c.core.cmp(&n.core) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match (&c.pre, &n.pre) {
            (None, Some(_)) => true,
            (Some(a), Some(b)) => compare_pre(a, b) == Ordering::Greater,
            _ => false,
        },
    }
}

pub fn pick(channel: Channel, releases: &[Release], current: &str) -> Option<Release> {
    releases
        .iter()
        .filter(|r| !r.draft && (channel == Channel::Nightly || !r.prerelease))
        .filter(|r| is_newer(&r.tag, current))
        .fold(None::<&Release>, |best, r| match best {
            Some(b) if !is_newer(&r.tag, &b.tag) => Some(b),
            _ => Some(r),
        })
        .cloned()
}

pub fn check(channel: Channel, current: &str) -> Result<Option<Release>, String> {
    let body = ureq::get(RELEASES_URL)
        .set("User-Agent", "belumi-launcher")
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| format!("update check failed: {e}"))?
        .into_string()
        .map_err(|e| format!("update check failed: {e}"))?;
    Ok(pick(channel, &parse_releases(&body), current))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, pre: bool) -> Release {
        Release {
            tag: tag.into(),
            url: format!("https://example.org/{tag}"),
            prerelease: pre,
            draft: false,
        }
    }

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("garbage", "0.1.0"));
        assert!(!is_newer("0.1.0", "garbage"));
        assert!(!is_newer("1.2", "0.1.0"));
    }

    #[test]
    fn a_release_beats_its_prereleases_and_prereleases_order_by_segment() {
        assert!(is_newer("0.2.0", "0.2.0-beta.3"));
        assert!(!is_newer("0.2.0-beta.3", "0.2.0"));
        assert!(is_newer("0.2.0-beta.10", "0.2.0-beta.9"));
        assert!(is_newer("0.2.0-rc.1", "0.2.0-beta.9"));
        assert!(is_newer("0.2.0-beta.1.1", "0.2.0-beta.1"));
    }

    #[test]
    fn stable_ignores_prereleases_and_drafts_nightly_takes_them() {
        let list = vec![
            release("v0.3.0-nightly.5", true),
            release("v0.2.0", false),
            Release { draft: true, ..release("v9.0.0", false) },
            release("v0.1.0", false),
        ];
        assert_eq!(pick(Channel::Stable, &list, "0.1.0").unwrap().tag, "v0.2.0");
        assert_eq!(pick(Channel::Nightly, &list, "0.1.0").unwrap().tag, "v0.3.0-nightly.5");
        assert_eq!(pick(Channel::Stable, &list, "0.2.0"), None);
        assert_eq!(pick(Channel::Nightly, &list, "0.3.0-nightly.5"), None);
    }

    #[test]
    fn github_release_lists_parse_and_garbage_gives_nothing() {
        let json = r#"[{"tag_name":"v1.0.0","html_url":"https://github.com/x/y/releases/tag/v1.0.0","prerelease":false,"draft":false,"name":"x","assets":[]}]"#;
        let r = parse_releases(json);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].tag, "v1.0.0");
        assert!(parse_releases("{\"message\":\"rate limited\"}").is_empty());
        assert!(parse_releases("").is_empty());
    }

    #[test]
    fn channels_parse_leniently() {
        assert_eq!(Channel::parse("Nightly"), Channel::Nightly);
        assert_eq!(Channel::parse("stable"), Channel::Stable);
        assert_eq!(Channel::parse("???"), Channel::Stable);
        assert_eq!(Channel::Nightly.name(), "nightly");
    }
}
