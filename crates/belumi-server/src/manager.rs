use crate::actions::{Action, Shared};
use crate::plugin::{Plugin, PluginError};
use belumi_plugin_api::{Event, EventResult, PluginManifest};
use std::path::Path;
use wasmtime::{Config, Engine};

pub struct PluginManager {
    engine: Engine,
    plugins: Vec<Plugin>,
    shared: Shared,
}

impl PluginManager {
    pub fn new() -> Result<Self, PluginError> {
        let mut cfg = Config::new();
        cfg.consume_fuel(true);
        Ok(Self {
            engine: Engine::new(&cfg)?,
            plugins: Vec::new(),
            shared: Shared::default(),
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

    pub fn try_command(&mut self, uuid: &str, line: &str) -> bool {
        let line = line.trim_start().trim_start_matches('/');
        let (name, args) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let name = name.to_ascii_lowercase();
        let Some(p) = self
            .plugins
            .iter_mut()
            .find(|p| !p.is_disabled() && p.manifest.commands.contains(&name))
        else {
            return false;
        };
        let event = Event::PluginCommand {
            uuid: uuid.to_owned(),
            name,
            args: args.trim().to_owned(),
        };
        if let Err(e) = p.dispatch(&event) {
            tracing::error!(target: "belumi::host", plugin = %p.manifest.name, "plugin disabled: {e}");
        }
        true
    }

    pub fn sync(&mut self, players: usize) -> Vec<Action> {
        self.shared.set_players(players);
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
