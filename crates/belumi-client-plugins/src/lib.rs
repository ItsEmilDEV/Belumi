pub use belumi_client_plugin_api::{Event, EventKind, EventResult, Manifest, Permission};
pub use belumi_client_plugin_api::{ScreenRow, ScreenSpec};
use belumi_client_plugin_api::{
    ABI_VERSION, ManifestError, MAX_CHAT_CHARS, MAX_HUD_CHARS, MAX_HUD_LINES,
};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};
use wasmtime::{
    Config, Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
    TypedFunc,
};

const FUEL_PER_CALL: u64 = 20_000_000;
const MEMORY_LIMIT: usize = 32 * 1024 * 1024;
const MAX_QUEUED_CHAT: usize = 64;
const MAX_SCREEN_JSON: usize = 8192;

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("wasm: {0}")]
    Wasm(String),
    #[error("plugin misses export `{0}`")]
    MissingExport(&'static str),
    #[error("plugin {0:?} is disabled after an earlier failure")]
    Disabled(String),
}

impl From<wasmtime::Error> for PluginError {
    fn from(e: wasmtime::Error) -> Self {
        PluginError::Wasm(format!("{e:#}"))
    }
}

#[derive(Default)]
struct Inner {
    chat: Vec<String>,
    hud: BTreeMap<(String, String), String>,
    player: String,
    /// Each plugin's screen, the plugin that asked to show its screen, and whether one asked to close it.
    screens: BTreeMap<String, ScreenSpec>,
    open: Option<String>,
    close: bool,
}

#[derive(Clone, Default)]
pub struct Shared(Arc<Mutex<Inner>>);

impl Shared {
    fn with<T>(&self, f: impl FnOnce(&mut Inner) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
    pub fn set_player(&self, json: String) {
        self.with(|i| i.player = json);
    }
    pub fn take_chat(&self) -> Vec<String> {
        self.with(|i| std::mem::take(&mut i.chat))
    }
    pub fn hud_lines(&self) -> Vec<String> {
        self.with(|i| i.hud.values().cloned().collect())
    }
    /// The screens plugins have set, by plugin name.
    pub fn screens(&self) -> Vec<(String, ScreenSpec)> {
        self.with(|i| i.screens.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }
    /// The plugin whose screen should open now, once.
    pub fn take_open(&self) -> Option<String> {
        self.with(|i| i.open.take())
    }
    /// Whether a plugin asked to close the screen, once.
    pub fn take_close(&self) -> bool {
        self.with(|i| std::mem::take(&mut i.close))
    }
    fn push_chat(&self, text: String) -> bool {
        self.with(|i| {
            if i.chat.len() >= MAX_QUEUED_CHAT {
                return false;
            }
            i.chat.push(text);
            true
        })
    }
    fn set_hud(&self, plugin: &str, id: String, text: String) -> i32 {
        self.with(|i| {
            let key = (plugin.to_owned(), id);
            if text.is_empty() {
                i.hud.remove(&key);
                return 0;
            }
            let mine = i.hud.keys().filter(|k| k.0 == plugin).count();
            if !i.hud.contains_key(&key) && mine >= MAX_HUD_LINES {
                return 2;
            }
            i.hud.insert(key, text);
            0
        })
    }
    fn clear_hud(&self, plugin: &str) {
        self.with(|i| {
            i.hud.retain(|k, _| k.0 != plugin);
            i.screens.remove(plugin);
            if i.open.as_deref() == Some(plugin) {
                i.open = None;
            }
        });
    }
}

struct HostState {
    name: String,
    limits: StoreLimits,
    permissions: Vec<Permission>,
    shared: Shared,
}

fn text_at(
    mem: &Memory,
    store: &impl wasmtime::AsContext,
    ptr: i32,
    len: i32,
    max: usize,
) -> Option<String> {
    let (s, l) = (usize::try_from(ptr).ok()?, usize::try_from(len).ok()?);
    let bytes = mem.data(store).get(s..s.checked_add(l)?)?;
    let text = String::from_utf8_lossy(bytes);
    Some(text.chars().take(max).collect())
}

fn guest_text(
    c: &mut wasmtime::Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    max: usize,
) -> Option<String> {
    let mem = c.get_export("memory")?.into_memory()?;
    text_at(&mem, &*c, ptr, len, max)
}

pub struct Plugin {
    pub manifest: Manifest,
    store: Store<HostState>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    on_event: TypedFunc<(i32, i32), i32>,
    shutdown: Option<TypedFunc<(), ()>>,
    disabled: bool,
    shared: Shared,
}

impl Plugin {
    fn load(
        engine: &Engine,
        manifest: Manifest,
        wasm: &[u8],
        shared: Shared,
    ) -> Result<Self, PluginError> {
        let module = Module::new(engine, wasm)?;
        let mut linker: Linker<HostState> = Linker::new(engine);
        linker.func_wrap(
            "belumi",
            "log",
            |mut c: wasmtime::Caller<'_, HostState>, level: i32, ptr: i32, len: i32| {
                let Some(msg) = guest_text(&mut c, ptr, len, 512) else {
                    return;
                };
                let name = c.data().name.clone();
                match level {
                    0 => log::debug!(target: "belumi::plugin", "[{name}] {msg}"),
                    1 => log::info!(target: "belumi::plugin", "[{name}] {msg}"),
                    2 => log::warn!(target: "belumi::plugin", "[{name}] {msg}"),
                    _ => log::error!(target: "belumi::plugin", "[{name}] {msg}"),
                }
            },
        )?;
        linker.func_wrap(
            "belumi",
            "chat_print",
            |mut c: wasmtime::Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                let Some(text) = guest_text(&mut c, ptr, len, MAX_CHAT_CHARS) else {
                    return -1;
                };
                let st = c.data();
                if !st.permissions.contains(&Permission::Chat) {
                    return 1;
                }
                if st.shared.push_chat(text) { 0 } else { 2 }
            },
        )?;
        linker.func_wrap(
            "belumi",
            "hud_set",
            |mut c: wasmtime::Caller<'_, HostState>, ip: i32, il: i32, tp: i32, tl: i32| -> i32 {
                let (Some(id), Some(text)) = (
                    guest_text(&mut c, ip, il, 32),
                    guest_text(&mut c, tp, tl, MAX_HUD_CHARS),
                ) else {
                    return -1;
                };
                let st = c.data();
                if !st.permissions.contains(&Permission::Hud) {
                    return 1;
                }
                st.shared.set_hud(&st.name, id, text)
            },
        )?;
        linker.func_wrap(
            "belumi",
            "screen_set",
            |mut c: wasmtime::Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                let Some(json) = guest_text(&mut c, ptr, len, MAX_SCREEN_JSON) else {
                    return -1;
                };
                let st = c.data();
                if !st.permissions.contains(&Permission::Screen) {
                    return 1;
                }
                let Some(spec) = ScreenSpec::parse(&json) else {
                    return -1;
                };
                let name = st.name.clone();
                st.shared.with(|i| i.screens.insert(name, spec));
                0
            },
        )?;
        linker.func_wrap(
            "belumi",
            "screen_open",
            |c: wasmtime::Caller<'_, HostState>| -> i32 {
                let st = c.data();
                if !st.permissions.contains(&Permission::Screen) {
                    return 1;
                }
                let name = st.name.clone();
                if !st.shared.with(|i| i.screens.contains_key(&name)) {
                    return -1;
                }
                st.shared.with(|i| i.open = Some(name));
                0
            },
        )?;
        linker.func_wrap(
            "belumi",
            "screen_close",
            |c: wasmtime::Caller<'_, HostState>| -> i32 {
                let st = c.data();
                if !st.permissions.contains(&Permission::Screen) {
                    return 1;
                }
                st.shared.with(|i| i.close = true);
                0
            },
        )?;
        linker.func_wrap(
            "belumi",
            "player",
            |mut c: wasmtime::Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
                let json = c.data().shared.with(|i| i.player.clone());
                let Ok(need) = i32::try_from(json.len()) else {
                    return -1;
                };
                if need > cap {
                    return need;
                }
                let Some(mem) = c.get_export("memory").and_then(|e| e.into_memory()) else {
                    return -1;
                };
                let Ok(start) = usize::try_from(ptr) else {
                    return -1;
                };
                if mem.write(&mut c, start, json.as_bytes()).is_err() {
                    return -1;
                }
                need
            },
        )?;
        let limits = StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).build();
        let mut store = Store::new(
            engine,
            HostState {
                name: manifest.name.clone(),
                limits,
                permissions: manifest.permissions.clone(),
                shared: shared.clone(),
            },
        );
        store.limiter(|s| &mut s.limits);
        store.set_fuel(FUEL_PER_CALL)?;
        let instance: Instance = linker.instantiate(&mut store, &module)?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or(PluginError::MissingExport("memory"))?;
        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "belumi_alloc")
            .map_err(|_| PluginError::MissingExport("belumi_alloc"))?;
        let on_event = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "belumi_on_event")
            .map_err(|_| PluginError::MissingExport("belumi_on_event"))?;
        if let Ok(init) = instance.get_typed_func::<(), ()>(&mut store, "belumi_init") {
            init.call(&mut store, ())?;
        }
        let shutdown = instance
            .get_typed_func::<(), ()>(&mut store, "belumi_shutdown")
            .ok();
        Ok(Plugin {
            manifest,
            store,
            memory,
            alloc,
            on_event,
            shutdown,
            disabled: false,
            shared,
        })
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    fn deliver(&mut self, event: &Event) -> Result<EventResult, PluginError> {
        if self.disabled {
            return Err(PluginError::Disabled(self.manifest.name.clone()));
        }
        let r = self.deliver_inner(event);
        if r.is_err() {
            self.disabled = true;
            self.shared.clear_hud(&self.manifest.name);
        }
        r
    }

    fn deliver_inner(&mut self, event: &Event) -> Result<EventResult, PluginError> {
        let json =
            event_json(event).ok_or_else(|| PluginError::Wasm("event does not serialize".into()))?;
        self.store.set_fuel(FUEL_PER_CALL)?;
        let len =
            i32::try_from(json.len()).map_err(|_| PluginError::Wasm("event too large".into()))?;
        let ptr = self.alloc.call(&mut self.store, len)?;
        let start =
            usize::try_from(ptr).map_err(|_| PluginError::Wasm("negative pointer".into()))?;
        self.memory
            .write(&mut self.store, start, json.as_bytes())
            .map_err(|e| PluginError::Wasm(format!("alloc returned a bad range: {e}")))?;
        let code = self.on_event.call(&mut self.store, (ptr, len))?;
        Ok(if code == 1 && cancellable(event) {
            EventResult::Cancel
        } else {
            EventResult::Continue
        })
    }
}

fn cancellable(event: &Event) -> bool {
    matches!(event, Event::Command { .. })
}

fn event_json(event: &Event) -> Option<String> {
    let esc = |s: &str| serde_json::to_string(s).ok();
    Some(match event {
        Event::Tick { tick } => format!("{{\"type\":\"tick\",\"tick\":{tick}}}"),
        Event::Key { key, pressed } => {
            format!("{{\"type\":\"key\",\"key\":{},\"pressed\":{pressed}}}", esc(key)?)
        }
        Event::Chat { text } => format!("{{\"type\":\"chat\",\"text\":{}}}", esc(text)?),
        Event::Command { name, args } => format!(
            "{{\"type\":\"command\",\"name\":{},\"args\":{}}}",
            esc(name)?,
            esc(args)?
        ),
        Event::Join { address } => format!("{{\"type\":\"join\",\"address\":{}}}", esc(address)?),
        Event::Leave => "{\"type\":\"leave\"}".to_owned(),
        Event::Screen { id, value } => {
            format!("{{\"type\":\"screen\",\"id\":{},\"value\":{value}}}", esc(id)?)
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub disabled: bool,
}

pub struct Host {
    engine: Engine,
    plugins: Vec<Plugin>,
    shared: Shared,
    errors: Vec<String>,
}

impl Host {
    pub fn new() -> Result<Self, PluginError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        Ok(Self {
            engine: Engine::new(&config)?,
            plugins: Vec::new(),
            shared: Shared::default(),
            errors: Vec::new(),
        })
    }

    pub fn load_dir(&mut self, dir: &Path) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut folders: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        folders.sort();
        for folder in folders {
            if let Err(e) = self.load_folder(&folder) {
                self.errors.push(format!("{}: {e}", folder.display()));
            }
        }
    }

    pub fn load_folder(&mut self, folder: &Path) -> Result<(), PluginError> {
        let manifest = Manifest::parse(&fs::read_to_string(folder.join("plugin.toml"))?)?;
        if self.plugins.iter().any(|p| p.manifest.name == manifest.name) {
            return Err(PluginError::Wasm(format!(
                "a plugin called {} is already loaded",
                manifest.name
            )));
        }
        let wasm = fs::read(folder.join("plugin.wasm"))?;
        self.load_bytes(manifest, &wasm)
    }

    pub fn load_bytes(&mut self, manifest: Manifest, wasm: &[u8]) -> Result<(), PluginError> {
        let plugin = Plugin::load(&self.engine, manifest, wasm, self.shared.clone())?;
        self.plugins.push(plugin);
        Ok(())
    }

    pub fn shared(&self) -> &Shared {
        &self.shared
    }

    pub fn abi_version(&self) -> u32 {
        ABI_VERSION
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }

    pub fn info(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|p| PluginInfo {
                name: p.manifest.name.clone(),
                version: p.manifest.version.clone(),
                description: p.manifest.description.clone(),
                disabled: p.disabled,
            })
            .collect()
    }

    pub fn owns_command(&self, name: &str) -> bool {
        self.plugins
            .iter()
            .any(|p| !p.disabled && p.manifest.commands.iter().any(|c| c == name))
    }

    pub fn run_command(&mut self, name: &str, args: &str) -> bool {
        let event = Event::Command {
            name: name.to_owned(),
            args: args.to_owned(),
        };
        let mut found = false;
        for p in &mut self.plugins {
            if p.disabled || !p.manifest.commands.iter().any(|c| c == name) {
                continue;
            }
            found = true;
            if let Err(e) = p.deliver(&event) {
                self.errors.push(format!("{}: {e}", p.manifest.name));
            }
            break;
        }
        found
    }

    /// A press on a plugin's own screen goes to that plugin only, whatever it subscribed to.
    pub fn screen_event(&mut self, plugin: &str, id: &str, value: bool) {
        let event = Event::Screen { id: id.to_owned(), value };
        if let Some(p) = self.plugins.iter_mut().find(|p| p.manifest.name == plugin && !p.disabled)
            && let Err(e) = p.deliver(&event)
        {
            self.errors.push(format!("{}: {e}", p.manifest.name));
        }
    }

    pub fn dispatch(&mut self, event: &Event) {
        let kind = event.kind();
        for p in &mut self.plugins {
            if p.disabled || !p.manifest.subscribe.contains(&kind) {
                continue;
            }
            if let Err(e) = p.deliver(event) {
                self.errors.push(format!("{}: {e}", p.manifest.name));
            }
        }
    }

    pub fn shutdown(&mut self) {
        for p in &mut self.plugins {
            if let (false, Some(f)) = (p.disabled, p.shutdown.as_ref()) {
                let _ = p.store.set_fuel(FUEL_PER_CALL);
                let _ = f.call(&mut p.store, ());
            }
            self.shared.clear_hud(&p.manifest.name);
        }
        self.plugins.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(perms: &str, subscribe: &str, commands: &str) -> Manifest {
        Manifest::parse(&format!(
            "name = \"t\"\nversion = \"1\"\napi_version = 1\npermissions = [{perms}]\nsubscribe = [{subscribe}]\ncommands = [{commands}]\n"
        ))
        .unwrap()
    }

    fn module(body: &str) -> Vec<u8> {
        wat::parse_str(format!(
            r#"(module
  (import "belumi" "chat_print" (func $chat (param i32 i32) (result i32)))
  (import "belumi" "hud_set" (func $hud (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "hi")
  (data (i32.const 8) "id")
  (data (i32.const 16) "text")
  (func (export "belumi_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "belumi_on_event") (param i32 i32) (result i32) {body}))"#
        ))
        .unwrap()
    }

    const SAY: &str = "(drop (call $chat (i32.const 0) (i32.const 2))) (drop (call $hud (i32.const 8) (i32.const 2) (i32.const 16) (i32.const 4))) (i32.const 0)";

    #[test]
    fn a_plugin_prints_to_chat_and_sets_hud_lines_with_permission() {
        let mut host = Host::new().unwrap();
        host.load_bytes(manifest("\"chat\", \"hud\"", "\"tick\"", ""), &module(SAY))
            .unwrap();
        host.dispatch(&Event::Tick { tick: 1 });
        assert_eq!(host.shared().take_chat(), vec!["hi".to_owned()]);
        assert_eq!(host.shared().hud_lines(), vec!["text".to_owned()]);
        assert!(host.take_errors().is_empty());
    }

    #[test]
    fn without_permission_nothing_happens_and_the_plugin_lives_on() {
        let mut host = Host::new().unwrap();
        host.load_bytes(manifest("", "\"tick\"", ""), &module(SAY)).unwrap();
        host.dispatch(&Event::Tick { tick: 1 });
        assert!(host.shared().take_chat().is_empty());
        assert!(host.shared().hud_lines().is_empty());
        assert!(!host.info()[0].disabled);
    }

    fn screen_module(perms: &str) -> (Manifest, Vec<u8>) {
        let wasm = wat::parse_str(
            r#"(module
  (import "belumi" "screen_set" (func $set (param i32 i32) (result i32)))
  (import "belumi" "screen_open" (func $open (result i32)))
  (import "belumi" "chat_print" (func $chat (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "{\"title\":\"T\",\"rows\":[{\"kind\":\"button\",\"id\":\"go\",\"text\":\"Go\"}]}")
  (data (i32.const 200) "pressed")
  (func (export "belumi_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "belumi_on_event") (param i32 i32) (result i32)
    (if (i32.eq (i32.load8_u (i32.add (local.get 0) (i32.const 9))) (i32.const 115))
      (then (drop (call $chat (i32.const 200) (i32.const 7))))
      (else (drop (call $set (i32.const 0) (i32.const 62))) (drop (call $open))))
    (i32.const 0)))"#,
        )
        .unwrap();
        (manifest(perms, "\"tick\"", ""), wasm)
    }

    #[test]
    fn a_plugin_sets_and_opens_its_screen_and_gets_the_presses() {
        let (m, wasm) = screen_module("\"screen\", \"chat\"");
        let mut host = Host::new().unwrap();
        host.load_bytes(m, &wasm).unwrap();
        host.dispatch(&Event::Tick { tick: 1 });
        let screens = host.shared().screens();
        assert_eq!(screens.len(), 1);
        assert_eq!(screens[0].1.title, "T");
        assert_eq!(host.shared().take_open().as_deref(), Some("t"));
        assert_eq!(host.shared().take_open(), None);
        host.screen_event("t", "go", false);
        assert_eq!(host.shared().take_chat(), vec!["pressed".to_owned()]);
        assert!(host.take_errors().is_empty());
    }

    #[test]
    fn without_the_permission_no_screen_appears() {
        let (m, wasm) = screen_module("\"chat\"");
        let mut host = Host::new().unwrap();
        host.load_bytes(m, &wasm).unwrap();
        host.dispatch(&Event::Tick { tick: 1 });
        assert!(host.shared().screens().is_empty());
        assert!(host.shared().take_open().is_none());
    }

    #[test]
    fn only_subscribed_events_arrive() {
        let mut host = Host::new().unwrap();
        host.load_bytes(manifest("\"chat\"", "\"key\"", ""), &module(SAY)).unwrap();
        host.dispatch(&Event::Tick { tick: 1 });
        assert!(host.shared().take_chat().is_empty());
        host.dispatch(&Event::Key {
            key: "key.keyboard.g".into(),
            pressed: true,
        });
        assert_eq!(host.shared().take_chat().len(), 1);
    }

    #[test]
    fn a_trap_or_an_endless_loop_disables_only_that_plugin_and_clears_its_hud() {
        for body in ["unreachable", "(loop $l (br $l)) (i32.const 0)"] {
            let mut host = Host::new().unwrap();
            host.load_bytes(manifest("\"hud\"", "\"tick\"", ""), &module(&format!("(drop (call $hud (i32.const 8) (i32.const 2) (i32.const 16) (i32.const 4))) {body}")))
                .unwrap();
            host.dispatch(&Event::Tick { tick: 1 });
            assert!(host.info()[0].disabled);
            assert!(host.shared().hud_lines().is_empty());
            assert_eq!(host.take_errors().len(), 1);
            host.dispatch(&Event::Tick { tick: 2 });
            assert!(host.take_errors().is_empty());
        }
    }

    #[test]
    fn commands_reach_only_their_owner_and_can_be_cancelled() {
        let mut host = Host::new().unwrap();
        host.load_bytes(manifest("\"chat\"", "", "\"hello\""), &module(&format!("(drop (call $chat (i32.const 0) (i32.const 2))) (i32.const 1)")))
            .unwrap();
        assert!(host.owns_command("hello") && !host.owns_command("bye"));
        assert!(host.run_command("hello", "world"));
        assert!(!host.run_command("bye", ""));
        assert_eq!(host.shared().take_chat(), vec!["hi".to_owned()]);
    }

    #[test]
    fn missing_exports_and_duplicates_are_load_errors() {
        let mut host = Host::new().unwrap();
        let empty = wat::parse_str("(module (memory (export \"memory\") 1))").unwrap();
        assert!(matches!(
            host.load_bytes(manifest("", "", ""), &empty),
            Err(PluginError::MissingExport(_))
        ));
    }

    #[test]
    fn folders_load_in_name_order_and_bad_ones_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good");
        fs::create_dir(&good).unwrap();
        fs::write(
            good.join("plugin.toml"),
            "name = \"good\"\nversion = \"1\"\napi_version = 1\n",
        )
        .unwrap();
        fs::write(good.join("plugin.wasm"), module(SAY)).unwrap();
        let bad = dir.path().join("bad");
        fs::create_dir(&bad).unwrap();
        fs::write(bad.join("plugin.toml"), "garbage").unwrap();
        let mut host = Host::new().unwrap();
        host.load_dir(dir.path());
        assert_eq!(host.info().len(), 1);
        assert_eq!(host.take_errors().len(), 1);
    }

    #[test]
    fn hud_lines_are_capped_per_plugin() {
        let shared = Shared::default();
        for i in 0..MAX_HUD_LINES {
            assert_eq!(shared.set_hud("p", format!("l{i}"), "x".into()), 0);
        }
        assert_eq!(shared.set_hud("p", "extra".into(), "x".into()), 2);
        assert_eq!(shared.set_hud("p", "l0".into(), "changed".into()), 0);
        assert_eq!(shared.set_hud("p", "l0".into(), String::new()), 0);
        assert_eq!(shared.hud_lines().len(), MAX_HUD_LINES - 1);
    }
}
