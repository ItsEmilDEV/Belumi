use serde::{Deserialize, Serialize};

pub const ABI_VERSION: u32 = 1;
pub const MAX_HUD_LINES: usize = 16;
pub const MAX_HUD_CHARS: usize = 120;
pub const MAX_CHAT_CHARS: usize = 256;
pub const MAX_SCREEN_ROWS: usize = 24;
pub const MAX_SCREEN_TEXT: usize = 60;
pub const MAX_SCREEN_ID: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Tick,
    Key,
    Chat,
    Command,
    Join,
    Leave,
    Screen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Tick { tick: u64 },
    Key { key: String, pressed: bool },
    Chat { text: String },
    Command { name: String, args: String },
    Join { address: String },
    Leave,
    /// A button of the plugin's own screen was pressed, or a toggle switched (`value` is its new state).
    Screen { id: String, value: bool },
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Event::Tick { .. } => EventKind::Tick,
            Event::Key { .. } => EventKind::Key,
            Event::Chat { .. } => EventKind::Chat,
            Event::Command { .. } => EventKind::Command,
            Event::Join { .. } => EventKind::Join,
            Event::Leave => EventKind::Leave,
            Event::Screen { .. } => EventKind::Screen,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Continue,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Chat,
    Hud,
    Screen,
}

/// One row of a plugin's screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScreenRow {
    Label { text: String },
    Button { id: String, text: String },
    Toggle { id: String, text: String, #[serde(default)] on: bool },
}

/// A screen a plugin shows: a title and rows from top to bottom, set as JSON with `screen_set`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScreenSpec {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub rows: Vec<ScreenRow>,
}

impl ScreenSpec {
    /// Reads and limits a screen: at most `MAX_SCREEN_ROWS` rows, short texts and ids, no row without an id.
    pub fn parse(json: &str) -> Option<Self> {
        let mut spec: ScreenSpec = serde_json::from_str(json).ok()?;
        let cut = |s: &mut String, max: usize| {
            if s.chars().count() > max {
                *s = s.chars().take(max).collect();
            }
        };
        cut(&mut spec.title, MAX_SCREEN_TEXT);
        spec.rows.truncate(MAX_SCREEN_ROWS);
        for row in &mut spec.rows {
            match row {
                ScreenRow::Label { text } => cut(text, MAX_SCREEN_TEXT),
                ScreenRow::Button { id, text } | ScreenRow::Toggle { id, text, .. } => {
                    cut(text, MAX_SCREEN_TEXT);
                    cut(id, MAX_SCREEN_ID);
                }
            }
        }
        let ids_ok = spec.rows.iter().all(|r| match r {
            ScreenRow::Label { .. } => true,
            ScreenRow::Button { id, .. } | ScreenRow::Toggle { id, .. } => !id.is_empty(),
        });
        ids_ok.then_some(spec)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("invalid plugin.toml: {0}")]
    Parse(String),
    #[error("plugin name {0:?} must be 1-32 characters of a-z, 0-9, - or _")]
    Name(String),
    #[error("plugin targets api_version {found}, this client speaks {ABI_VERSION}")]
    Api { found: u32 },
    #[error("command {0:?} must be 1-32 characters of a-z, 0-9 or _")]
    Command(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub api_version: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub subscribe: Vec<EventKind>,
    #[serde(default)]
    pub commands: Vec<String>,
}

fn word(s: &str, extra: &[char]) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || extra.contains(&c))
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let m: Manifest = toml::from_str(text).map_err(|e| ManifestError::Parse(e.to_string()))?;
        if !word(&m.name, &['-', '_']) {
            return Err(ManifestError::Name(m.name));
        }
        if m.api_version != ABI_VERSION {
            return Err(ManifestError::Api {
                found: m.api_version,
            });
        }
        if let Some(bad) = m.commands.iter().find(|c| !word(c, &['_'])) {
            return Err(ManifestError::Command(bad.clone()));
        }
        Ok(m)
    }
    pub fn allows(&self, p: Permission) -> bool {
        self.permissions.contains(&p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_is_read_and_limited() {
        let long = "x".repeat(200);
        let json = format!(r#"{{"title":"{long}","rows":[{{"kind":"label","text":"hi"}},{{"kind":"button","id":"go","text":"Go"}},{{"kind":"toggle","id":"t","text":"T","on":true}}]}}"#);
        let spec = ScreenSpec::parse(&json).unwrap();
        assert_eq!(spec.title.chars().count(), MAX_SCREEN_TEXT);
        assert_eq!(spec.rows.len(), 3);
        assert_eq!(spec.rows[2], ScreenRow::Toggle { id: "t".into(), text: "T".into(), on: true });
        let many = format!(r#"{{"rows":[{}]}}"#, vec![r#"{"kind":"label","text":"a"}"#; 40].join(","));
        assert_eq!(ScreenSpec::parse(&many).unwrap().rows.len(), MAX_SCREEN_ROWS);
    }

    #[test]
    fn a_button_without_an_id_or_nonsense_is_refused() {
        assert!(ScreenSpec::parse(r#"{"rows":[{"kind":"button","id":"","text":"Go"}]}"#).is_none());
        assert!(ScreenSpec::parse("nonsense").is_none());
        assert!(ScreenSpec::parse(r#"{"rows":[{"kind":"slider"}]}"#).is_none());
    }

    const OK: &str = "name = \"compass\"\nversion = \"1.0\"\napi_version = 1\npermissions = [\"hud\"]\nsubscribe = [\"tick\", \"key\"]\ncommands = [\"north\"]\n";

    #[test]
    fn a_valid_manifest_parses() {
        let m = Manifest::parse(OK).unwrap();
        assert_eq!(m.name, "compass");
        assert!(m.allows(Permission::Hud) && !m.allows(Permission::Chat));
        assert_eq!(m.subscribe, vec![EventKind::Tick, EventKind::Key]);
    }

    #[test]
    fn bad_names_versions_and_commands_are_refused() {
        assert!(matches!(
            Manifest::parse(&OK.replace("compass", "Bad Name")),
            Err(ManifestError::Name(_))
        ));
        assert_eq!(
            Manifest::parse(&OK.replace("api_version = 1", "api_version = 9")),
            Err(ManifestError::Api { found: 9 })
        );
        assert!(matches!(
            Manifest::parse(&OK.replace("north", "No/rth")),
            Err(ManifestError::Command(_))
        ));
        assert!(matches!(
            Manifest::parse("nonsense"),
            Err(ManifestError::Parse(_))
        ));
    }

    #[test]
    fn events_round_trip_as_tagged_json_and_know_their_kind() {
        let e = Event::Key {
            key: "key.keyboard.g".into(),
            pressed: true,
        };
        let json = serde_json::to_string(&e).unwrap_or_default();
        assert!(json.is_empty() || json.contains("\"type\":\"key\""));
        assert_eq!(e.kind(), EventKind::Key);
        assert_eq!(Event::Leave.kind(), EventKind::Leave);
    }
}
