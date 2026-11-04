use crate::actions::{Action, Shared};
use crate::plugin::{Plugin, PluginError};
use belumi_plugin_api::{Event, EventResult, PluginManifest};
use std::path::{Path, PathBuf};
use wasmtime::{Config, Engine};

pub struct PluginManager {
    engine: Engine,
    plugins: Vec<Plugin>,
    shared: Shared,
    dir: Option<PathBuf>,
}

/// What became of a chat line that might be a plugin command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOutcome {
    /// No plugin owns this command.
    NotPlugin,
    /// A plugin owns it, but the player's operator level is too low.
    Denied,
    Ran,
}

impl PluginManager {
    pub fn new() -> Result<Self, PluginError> {
        let mut cfg = Config::new();
        cfg.consume_fuel(true);
        Ok(Self {
            engine: Engine::new(&cfg)?,
            plugins: Vec::new(),
            shared: Shared::default(),
            dir: None,
        })
    }

    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect()
    }

    pub fn load_bytes(&mut self, manifest: PluginManifest, wasm: &[u8]) -> Result<(), PluginError> {
        let p = Plugin::load(&self.engine, manifest, wasm, self.shared.clone())?;
        self.plugins.push(p);
        Ok(())
    }

    pub fn load_dir(&mut self, dir: &Path) -> Result<usize, PluginError> {
        self.dir = Some(dir.to_owned());
        let mut found: Vec<(PluginManifest, Vec<u8>)> = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let folder = entry?.path();
            let manifest_path = folder.join("plugin.toml");
            if !manifest_path.is_file() {
                continue;
            }
            let read = || -> Result<_, PluginError> {
                let m = PluginManifest::parse(&std::fs::read_to_string(&manifest_path)?)?;
                let wasm = std::fs::read(folder.join(&m.main))?;
                Ok((m, wasm))
            };
            match read() {
                Ok(x) => found.push(x),
                Err(e) => {
                    tracing::error!(target: "belumi::host", path = %folder.display(), "plugin skipped: {e}")
                }
            }
        }
        let mut loaded = 0;
        let mut pending = found;
        while !pending.is_empty() {
            let before = pending.len();
            let mut rest = Vec::new();
            for (m, wasm) in pending {
                if m.depends
                    .iter()
                    .all(|d| self.plugin_names().contains(&d.as_str()))
                {
                    let name = m.name.clone();
                    match self.load_bytes(m, &wasm) {
                        Ok(()) => loaded += 1,
                        Err(e) => {
                            tracing::error!(target: "belumi::host", plugin = %name, "plugin failed to load: {e}")
                        }
                    }
                } else {
                    rest.push((m, wasm));
                }
            }
            if rest.len() == before {
                for (m, _) in &rest {
                    tracing::error!(target: "belumi::host", plugin = %m.name, "plugin skipped: missing or cyclic dependency");
                }
                break;
            }
            pending = rest;
        }
        Ok(loaded)
    }

    pub fn command_names(&self) -> Vec<String> {
        self.plugins
            .iter()
            .filter(|p| !p.is_disabled())
            .flat_map(|p| p.manifest.commands.iter().cloned())
            .collect()
    }

    /// Stops all plugins, reads the plugin folder again and starts them.
    /// Returns how many plugins are loaded afterwards.
    pub fn reload(&mut self) -> Result<usize, PluginError> {
        let Some(dir) = self.dir.clone() else {
            return Ok(0);
        };
        self.emit(&Event::ServerStop);
        self.plugins.clear();
        let n = self.load_dir(&dir)?;
        self.emit(&Event::ServerStart);
        Ok(n)
    }

    /// Runs `line` as a plugin command when a plugin owns it and `level`
    /// (the player's operator level, 0-4) reaches the command's minimum.
    pub fn try_command(&mut self, uuid: &str, level: u8, line: &str) -> CommandOutcome {
        let line = line.trim_start().trim_start_matches('/');
        let (name, args) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let name = name.to_ascii_lowercase();
        let Some(p) = self
            .plugins
            .iter_mut()
            .find(|p| !p.is_disabled() && p.manifest.commands.contains(&name))
        else {
            return CommandOutcome::NotPlugin;
        };
        if level < p.manifest.command_levels.get(&name).copied().unwrap_or(0) {
            self.shared.push(Action::Message {
                uuid: uuid.to_owned(),
                text: "You do not have permission to use this command.".into(),
            });
            return CommandOutcome::Denied;
        }
        let event = Event::PluginCommand {
            uuid: uuid.to_owned(),
            name,
            args: args.trim().to_owned(),
        };
        if let Err(e) = p.dispatch(&event) {
            tracing::error!(target: "belumi::host", plugin = %p.manifest.name, "plugin disabled: {e}");
        }
        CommandOutcome::Ran
    }

    /// The online players changed: `(uuid, name)` of everyone online.
    pub fn set_roster(&mut self, players: &[(String, String)]) {
        self.shared.set_players(players);
    }

    /// Queues a chat message for one player.
    pub fn notify(&self, uuid: &str, text: &str) {
        self.shared.push(Action::Message {
            uuid: uuid.to_owned(),
            text: text.to_owned(),
        });
    }

    /// Per tick: the actions the plugins queued since the last call.
    pub fn sync(&mut self) -> Vec<Action> {
        self.shared.take()
    }

    pub fn emit(&mut self, event: &Event) -> EventResult {
        let mut result = EventResult::Continue;
        for p in self
            .plugins
            .iter_mut()
            .filter(|p| !p.is_disabled() && p.wants(event))
        {
            match p.dispatch(event) {
                Ok(EventResult::Cancel) => result = EventResult::Cancel,
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(target: "belumi::host", plugin = %p.manifest.name, "plugin disabled: {e}")
                }
            }
        }
        result
    }
}
