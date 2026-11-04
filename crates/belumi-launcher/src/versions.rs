#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    Native,
    Translated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameVersion {
    pub id: &'static str,
    pub protocol: i32,
    pub support: Support,
}

pub const VERSIONS: &[GameVersion] = &[
    GameVersion {
        id: "26.3",
        protocol: 777,
        support: Support::Native,
    },
    GameVersion {
        id: "26.2",
        protocol: 776,
        support: Support::Translated,
    },
];

pub const AUTO: &str = "auto";

pub fn find(id: &str) -> Option<&'static GameVersion> {
    VERSIONS.iter().find(|v| v.id == id.trim())
}

pub fn protocol_arg(choice: &str) -> String {
    match find(choice) {
        Some(v) if v.support == Support::Native => "native".to_owned(),
        Some(v) => v.protocol.to_string(),
        None => AUTO.to_owned(),
    }
}

pub fn label(choice: &str) -> String {
    match find(choice) {
        Some(v) => match v.support {
            Support::Native => format!("{} (native)", v.id),
            Support::Translated => format!("{} (translated)", v.id),
        },
        None => "Auto-detect".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_version_is_first_and_native_the_rest_are_translated() {
        assert_eq!(VERSIONS[0].support, Support::Native);
        assert!(VERSIONS[1..].iter().all(|v| v.support == Support::Translated));
        assert!(VERSIONS.windows(2).all(|w| w[0].protocol > w[1].protocol));
    }

    #[test]
    fn choices_map_to_the_clients_protocol_option() {
        assert_eq!(protocol_arg("auto"), "auto");
        assert_eq!(protocol_arg(""), "auto");
        assert_eq!(protocol_arg("26.3"), "native");
        assert_eq!(protocol_arg("26.2"), "776");
        assert_eq!(protocol_arg("1.8"), "auto");
    }

    #[test]
    fn labels_say_how_a_version_is_played() {
        assert_eq!(label("26.3"), "26.3 (native)");
        assert_eq!(label("26.2"), "26.2 (translated)");
        assert_eq!(label("auto"), "Auto-detect");
    }
}
