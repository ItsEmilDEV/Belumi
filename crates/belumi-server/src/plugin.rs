use crate::actions::{Action, Shared};
use belumi_plugin_api::{Event, EventResult, Permission, PluginManifest};
use wasmtime::{
    Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

const FUEL_PER_CALL: u64 = 50_000_000;
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error(transparent)]
    Manifest(#[from] belumi_plugin_api::ManifestError),
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

struct HostState {
    name: String,
    limits: StoreLimits,
    permissions: Vec<Permission>,
    shared: Shared,
}

pub struct Plugin {
    pub manifest: PluginManifest,
    store: Store<HostState>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    on_event: TypedFunc<(i32, i32), i32>,
    disabled: bool,
}

fn read_str(mem: &Memory, store: &impl wasmtime::AsContext, ptr: i32, len: i32) -> Option<String> {
    let data = mem.data(store);
    let (s, l) = (usize::try_from(ptr).ok()?, usize::try_from(len).ok()?);
    let bytes = data.get(s..s.checked_add(l)?)?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn host_str(c: &mut wasmtime::Caller<'_, HostState>, ptr: i32, len: i32) -> Option<String> {
    let mem = c.get_export("memory")?.into_memory()?;
    read_str(&mem, &*c, ptr, len)
}

fn queue(c: &mut wasmtime::Caller<'_, HostState>, need: Permission, action: Action) -> i32 {
    let st = c.data();
    if !st.permissions.contains(&need) {
        tracing::warn!(target: "belumi::plugin", plugin = %st.name, "refused: needs permission {need:?}");
        return 1;
    }
    if st.shared.push(action) { 0 } else { 2 }
}

impl Plugin {
    pub(crate) fn load(
        engine: &Engine,
        manifest: PluginManifest,
        wasm: &[u8],
        shared: Shared,
    ) -> Result<Self, PluginError> {
        let module = Module::new(engine, wasm)?;
        let mut linker: Linker<HostState> = Linker::new(engine);
        linker.func_wrap(
            "belumi",
            "log",
            |mut c: wasmtime::Caller<'_, HostState>, level: i32, ptr: i32, len: i32| {
                let Some(mem) = c.get_export("memory").and_then(|e| e.into_memory()) else {
                    return;
                };
                let Some(msg) = read_str(&mem, &c, ptr, len) else {
                    return;
                };
                let name = c.data().name.clone();
                match level {
                    0 => tracing::debug!(target: "belumi::plugin", plugin = %name, "{msg}"),
                    1 => tracing::info!(target: "belumi::plugin", plugin = %name, "{msg}"),
                    2 => tracing::warn!(target: "belumi::plugin", plugin = %name, "{msg}"),
                    _ => tracing::error!(target: "belumi::plugin", plugin = %name, "{msg}"),
                }
            },
        )?;
        linker.func_wrap(
            "belumi",
            "player_count",
            |c: wasmtime::Caller<'_, HostState>| c.data().shared.players(),
        )?;
        linker.func_wrap(
            "belumi",
            "players",
            |mut c: wasmtime::Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
                let json = c.data().shared.roster();
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
        linker.func_wrap(
            "belumi",
            "broadcast",
            |mut c: wasmtime::Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                let Some(text) = host_str(&mut c, ptr, len) else {
                    return -1;
                };
                queue(&mut c, Permission::Messaging, Action::Broadcast(text))
            },
        )?;
        linker.func_wrap(
            "belumi",
            "send_message",
            |mut c: wasmtime::Caller<'_, HostState>, up: i32, ul: i32, tp: i32, tl: i32| -> i32 {
                let (Some(uuid), Some(text)) = (host_str(&mut c, up, ul), host_str(&mut c, tp, tl))
                else {
                    return -1;
                };
                queue(
                    &mut c,
                    Permission::Messaging,
                    Action::Message { uuid, text },
                )
            },
        )?;
        linker.func_wrap(
            "belumi",
            "kick",
            |mut c: wasmtime::Caller<'_, HostState>, up: i32, ul: i32, rp: i32, rl: i32| -> i32 {
                let (Some(uuid), Some(reason)) =
                    (host_str(&mut c, up, ul), host_str(&mut c, rp, rl))
                else {
                    return -1;
                };
                queue(&mut c, Permission::Kick, Action::Kick { uuid, reason })
            },
        )?;
        let limits = StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).build();
        let mut store = Store::new(
            engine,
            HostState {
                name: manifest.name.clone(),
                limits,
                permissions: manifest.permissions.clone(),
                shared,
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
        Ok(Plugin {
            manifest,
            store,
            memory,
            alloc,
            on_event,
            disabled: false,
        })
    }

    pub fn wants(&self, event: &Event) -> bool {
        self.manifest.subscribe.contains(&event.kind())
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn dispatch(&mut self, event: &Event) -> Result<EventResult, PluginError> {
        if self.disabled {
            return Err(PluginError::Disabled(self.manifest.name.clone()));
        }
        let r = self.dispatch_inner(event);
        if r.is_err() {
            self.disabled = true;
        }
        r
    }

    fn dispatch_inner(&mut self, event: &Event) -> Result<EventResult, PluginError> {
        let json = serde_json::to_vec(event).expect("events serialize");
        self.store.set_fuel(FUEL_PER_CALL)?;
        let len =
            i32::try_from(json.len()).map_err(|_| PluginError::Wasm("event too large".into()))?;
        let ptr = self.alloc.call(&mut self.store, len)?;
        let start =
            usize::try_from(ptr).map_err(|_| PluginError::Wasm("negative pointer".into()))?;
        self.memory
            .write(&mut self.store, start, &json)
            .map_err(|e| PluginError::Wasm(format!("alloc returned a bad range: {e}")))?;
        let code = self.on_event.call(&mut self.store, (ptr, len))?;
        Ok(if code == 1 && event.cancellable() {
            EventResult::Cancel
        } else {
            EventResult::Continue
        })
    }
}
