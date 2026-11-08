use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    De,
}

impl Lang {
    /// `auto` follows the system language; anything unknown is English.
    pub fn parse(setting: &str, system: &str) -> Self {
        match setting.trim().to_ascii_lowercase().as_str() {
            "de" => Self::De,
            "en" => Self::En,
            _ => {
                if system.to_ascii_lowercase().starts_with("de") {
                    Self::De
                } else {
                    Self::En
                }
            }
        }
    }

}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == 1 { Lang::De } else { Lang::En }
}

pub fn system_language() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

const GERMAN: &[(&str, &str)] = &[
    ("Play", "Spielen"),
    ("Uninstall", "Deinstallieren"),
    ("Uninstall...", "Deinstallieren..."),
    ("Only cache and usage data", "Nur Cache und Nutzungsdaten"),
    ("Only the app", "Nur die App"),
    ("Everything", "Alles"),
    ("Chunk cache, maps, heads, logs, crash reports and statistics. Settings, accounts and worlds stay.", "Chunk-Cache, Karten, Köpfe, Logs, Absturzberichte und Statistiken. Einstellungen, Konten und Welten bleiben."),
    ("Program files and menu entries. Settings, accounts, worlds and data stay.", "Programmdateien und Menü-Einträge. Einstellungen, Konten, Welten und Daten bleiben."),
    ("The app and all its data: settings, accounts, prepared resources and caches.", "Die App und alle ihre Daten: Einstellungen, Konten, vorbereitete Ressourcen und Caches."),
    ("Also my worlds and screenshots (in the game folder)", "Auch meine Welten und Screenshots (im Spielordner)"),
    ("Nothing to remove.", "Nichts zu entfernen."),
    ("This deletes, for good:", "Das wird endgültig gelöscht:"),
    ("The program files, with the installer's uninstaller", "Die Programmdateien, mit dem Deinstallationsprogramm des Installers"),
    ("Yes, delete", "Ja, löschen"),
    ("Removed {} items.", "{} Einträge entfernt."),
    ("Removed {} items, {} failed (first: {}: {}). Installed by a package manager? Remove it with that.", "{} Einträge entfernt, {} fehlgeschlagen (erster: {}: {}). Per Paketmanager installiert? Dann damit entfernen."),
    ("Account", "Konto"),
    ("Game log", "Spiel-Log"),
    ("Settings", "Einstellungen"),
    ("Worlds", "Welten"),
    ("Servers", "Server"),
    ("Add account", "Konto hinzufügen"),
    ("Renew now", "Jetzt erneuern"),
    ("Use my Prism account", "Mein Prism-Konto nutzen"),
    ("Sign out", "Abmelden"),
    ("Saved accounts", "Gespeicherte Konten"),
    ("Use", "Nutzen"),
    ("Remove", "Entfernen"),
    ("active", "aktiv"),
    ("Keep using Prism's login automatically", "Prisms Anmeldung automatisch weiter nutzen"),
    ("Cancel", "Abbrechen"),
    ("Cancelled.", "Abgebrochen."),
    ("Refresh", "Aktualisieren"),
    ("Main menu", "Hauptmenü"),
    ("Play offline as {}", "Offline spielen als {}"),
    ("Sign in first, or play offline", "Zuerst anmelden oder offline spielen"),
    ("Starts the game without joining the selected server", "Startet das Spiel, ohne dem gewählten Server beizutreten"),
    ("Signs in another Microsoft account; the others stay saved", "Meldet ein weiteres Microsoft-Konto an; die anderen bleiben gespeichert"),
    ("Gets a fresh token for the active account", "Holt ein frisches Token für das aktive Konto"),
    ("Not signed in", "Nicht angemeldet"),
    ("Signed in as {}.", "Angemeldet als {}."),
    ("Signed out.", "Abgemeldet."),
    ("Idle", "Bereit"),
    ("Working", "Arbeitet"),
    ("Running", "Läuft"),
    ("Running...", "Läuft..."),
    ("Game started.", "Spiel gestartet."),
    ("Look", "Aussehen"),
    ("Language", "Sprache"),
    ("Azure client ID", "Azure-Client-ID"),
    ("Game binary", "Spieldatei"),
    ("Resources", "Ressourcen"),
    ("Offline name", "Offline-Name"),
    ("Prism accounts.json", "Prism accounts.json"),
    ("Check for updates at start", "Beim Start nach Updates suchen"),
    ("Open the release page", "Release-Seite öffnen"),
    ("Update available: {}", "Update verfügbar: {}"),
    ("Asks GitHub's public release list; nothing is installed automatically.", "Fragt die öffentliche Release-Liste von GitHub ab; es wird nichts automatisch installiert."),
    ("Nothing yet. The game writes to {}", "Noch nichts. Das Spiel schreibt nach {}"),
    ("Open {} and enter:", "Öffne {} und gib ein:"),
    ("No server yet: the game starts in its main menu.", "Noch kein Server: das Spiel startet im Hauptmenü."),
    ("The sign-in is kept in your desktop's keyring when there is one (secret-tool), otherwise in an owner-only file.", "Die Anmeldung liegt im Schlüsselbund deines Desktops, falls vorhanden (secret-tool), sonst in einer Datei nur für dich."),
    ("Name", "Name"),
    ("Add", "Hinzufügen"),
    ("Auto-detect", "Automatisch"),
    ("Online", "Online"),
    ("Offline", "Offline"),
    ("{} of {} players", "{} von {} Spielern"),
    ("Last played {}", "Zuletzt gespielt {}"),
    ("Start in this world", "In dieser Welt starten"),
    ("No worlds found in {}", "Keine Welten gefunden in {}"),
    ("Launch options", "Starteinstellungen"),
    ("Fullscreen", "Vollbild"),
    ("Window size", "Fenstergröße"),
    ("Graphics API", "Grafik-API"),
    ("After the game starts", "Nach dem Spielstart"),
    ("Keep the launcher open", "Launcher offen lassen"),
    ("Minimize the launcher", "Launcher minimieren"),
    ("Close the launcher", "Launcher schließen"),
    ("Account for this server", "Konto für diesen Server"),
    ("Active account", "Aktives Konto"),
    ("Everything fine", "Alles in Ordnung"),
    ("Check", "Prüfung"),
    ("Open folder", "Ordner öffnen"),
    ("Copy report", "Bericht kopieren"),
    ("Pack logs for support", "Logs für Support packen"),
    ("The game stopped unexpectedly.", "Das Spiel wurde unerwartet beendet."),
    ("Run diagnose", "Diagnose starten"),
    ("Shader pack", "Shader-Pack"),
    ("Quality", "Qualität"),
    ("None", "Keins"),
    ("Drop a shader pack (.zip) here to add it", "Ein Shader-Pack (.zip) hier ablegen, um es hinzuzufügen"),
    ("Add server", "Server hinzufügen"),
    ("Added the shader pack {}.", "Shader-Pack {} hinzugefügt."),
    ("Auto", "Automatisch"),
    ("Choose who plays and where", "Wähle, wer spielt und wo"),
    ("Close", "Schließen"),
    ("Copy", "Kopieren"),
    ("Game file", "Spieldatei"),
    ("Game files", "Spieldateien"),
    ("Game folder", "Spielordner"),
    ("Graphics driver", "Grafiktreiber"),
    ("Keep open", "Offen lassen"),
    ("Log copied.", "Log kopiert."),
    ("Look, game files and how the game starts", "Aussehen, Spieldateien und wie das Spiel startet"),
    ("Minimize", "Minimieren"),
    ("No other accounts yet.", "Noch keine weiteren Konten."),
    ("Other accounts", "Weitere Konten"),
    ("Play offline", "Offline spielen"),
    ("Report copied.", "Bericht kopiert."),
    ("Saved {}", "Gespeichert: {}"),
    ("Shader packs", "Shader-Packs"),
    ("Sign in first (or play offline).", "Zuerst anmelden (oder offline spielen)."),
    ("Start", "Starten"),
    ("Asking Microsoft for a sign-in code...", "Frage Microsoft nach einem Anmeldecode..."),
    ("Renewing the sign-in of {}...", "Erneuere die Anmeldung von {}..."),
    ("The saved sign-in could not be read from the keyring. Sign in again.", "Die gespeicherte Anmeldung ließ sich nicht aus dem Schlüsselbund lesen. Melde dich neu an."),
    ("Diagnose is running; the game opens a few times.", "Diagnose läuft; das Spiel öffnet sich mehrmals."),
    ("Report ready: {}", "Bericht fertig: {}"),
    ("Runs benchmarks and screenshots in a scratch folder and collects a report to send for support.", "Führt Benchmarks und Screenshots in einem Arbeitsordner aus und sammelt einen Bericht für den Support."),
    ("Something went wrong inside the launcher. Try again.", "Im Launcher ist etwas schiefgelaufen. Versuch es nochmal."),
    ("Starts Minecraft as {} on {}", "Startet Minecraft als {} auf {}"),
    ("Starts Minecraft {} as {} on {}", "Startet Minecraft {} als {} auf {}"),
    ("Switch", "Wechseln"),
    ("Switching to the account of this server...", "Wechsle zum Konto dieses Servers..."),
    ("Updates", "Updates"),
    ("Version", "Version"),
    ("What the game wrote the last time it ran", "Was das Spiel beim letzten Start geschrieben hat"),
    ("Who plays, and how the sign-in is kept", "Wer spielt und wie die Anmeldung gespeichert wird"),
    ("play offline or sign in under Account", "offline spielen oder unter Konto anmelden"),
    ("saved, not renewed yet", "gespeichert, noch nicht erneuert"),
    ("signed in", "angemeldet"),
    ("via Prism · token valid for {} h", "über Prism · Token gültig für {} h"),
    ("via the Belumi app · token valid for {} h", "über die Belumi-App · Token gültig für {} h"),
    ("{} is not a shader pack (.zip).", "{} ist kein Shader-Pack (.zip)."),
];

fn table() -> &'static HashMap<&'static str, &'static str> {
    static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| GERMAN.iter().copied().collect())
}

/// The text in the current language; English is the key and the fallback.
pub fn tr(english: &str) -> String {
    if current() == Lang::De
        && let Some(german) = table().get(english)
    {
        return (*german).to_owned();
    }
    english.to_owned()
}

/// `tr` with each `{}` replaced by the next argument.
pub fn trf(english: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut rest = tr(english);
    let mut args = args.iter();
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        out.push_str(args.next().copied().unwrap_or(""));
        rest = rest[at + 2..].to_owned();
    }
    out.push_str(&rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_language_follows_the_setting_then_the_system() {
        assert_eq!(Lang::parse("auto", "de_DE.UTF-8"), Lang::De);
        assert_eq!(Lang::parse("auto", "en_US.UTF-8"), Lang::En);
        assert_eq!(Lang::parse("auto", ""), Lang::En);
        assert_eq!(Lang::parse("de", "en_US"), Lang::De);
        assert_eq!(Lang::parse("en", "de_DE"), Lang::En);
        assert_eq!(Lang::parse("klingon", "de"), Lang::De);
    }

    #[test]
    fn every_german_text_is_filled_unique_and_keeps_its_placeholders() {
        let mut seen = std::collections::HashSet::new();
        for (en, de) in GERMAN {
            assert!(seen.insert(*en), "duplicate key {en}");
            assert!(!de.trim().is_empty(), "empty German for {en}");
            assert_eq!(en.matches("{}").count(), de.matches("{}").count(), "placeholders of {en}");
        }
    }

    #[test]
    fn texts_switch_and_unknown_ones_stay_english() {
        set(Lang::De);
        assert_eq!(tr("Play"), "Spielen");
        assert_eq!(tr("Not in the table"), "Not in the table");
        assert_eq!(trf("Signed in as {}.", &["Alex"]), "Angemeldet als Alex.");
        assert_eq!(trf("{} of {} players", &["3", "20"]), "3 von 20 Spielern");
        set(Lang::En);
        assert_eq!(tr("Play"), "Play");
        assert_eq!(trf("Play offline as {}", &["Steve"]), "Play offline as Steve");
    }
}
