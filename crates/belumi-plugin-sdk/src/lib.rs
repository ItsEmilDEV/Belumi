#![allow(unsafe_code)]

pub use belumi_plugin_api::{ABI_VERSION, Event, EventResult};

pub trait Plugin: Default {
    fn init(&mut self) {}
    fn on_event(&mut self, event: &Event) -> EventResult;
}

pub mod host {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HostError {
        Denied,
        QueueFull,
        Invalid,
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "belumi")]
    unsafe extern "C" {
        fn log(level: i32, ptr: *const u8, len: i32);
        fn broadcast(ptr: *const u8, len: i32) -> i32;
        fn send_message(up: *const u8, ul: i32, tp: *const u8, tl: i32) -> i32;
        fn kick(up: *const u8, ul: i32, rp: *const u8, rl: i32) -> i32;
        fn player_count() -> i32;
        fn players(ptr: *mut u8, cap: i32) -> i32;
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod stub {
        pub unsafe fn log(_: i32, _: *const u8, _: i32) {}
        pub unsafe fn broadcast(_: *const u8, _: i32) -> i32 {
            0
        }
        pub unsafe fn send_message(_: *const u8, _: i32, _: *const u8, _: i32) -> i32 {
            0
        }
        pub unsafe fn kick(_: *const u8, _: i32, _: *const u8, _: i32) -> i32 {
            0
        }
        pub unsafe fn player_count() -> i32 {
            0
        }
        pub unsafe fn players(_: *mut u8, _: i32) -> i32 {
            2
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    use stub::*;

    fn result(code: i32) -> Result<(), HostError> {
        match code {
            0 => Ok(()),
            1 => Err(HostError::Denied),
            2 => Err(HostError::QueueFull),
            _ => Err(HostError::Invalid),
        }
    }

    fn len(s: &str) -> i32 {
        i32::try_from(s.len()).unwrap_or(i32::MAX)
    }

    pub fn debug(msg: &str) {
        unsafe { log(0, msg.as_ptr(), len(msg)) }
    }
    pub fn info(msg: &str) {
        unsafe { log(1, msg.as_ptr(), len(msg)) }
    }
    pub fn warn(msg: &str) {
        unsafe { log(2, msg.as_ptr(), len(msg)) }
    }
    pub fn error(msg: &str) {
        unsafe { log(3, msg.as_ptr(), len(msg)) }
    }

    pub fn broadcast_message(text: &str) -> Result<(), HostError> {
        result(unsafe { broadcast(text.as_ptr(), len(text)) })
    }

    pub fn send_message_to(uuid: &str, text: &str) -> Result<(), HostError> {
        result(unsafe { send_message(uuid.as_ptr(), len(uuid), text.as_ptr(), len(text)) })
    }

    pub fn kick_player(uuid: &str, reason: &str) -> Result<(), HostError> {
        result(unsafe { kick(uuid.as_ptr(), len(uuid), reason.as_ptr(), len(reason)) })
    }

    pub fn online_players() -> i32 {
        unsafe { player_count() }
    }

    /// An online player.
    #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
    pub struct Player {
        pub uuid: String,
        pub name: String,
    }

    /// Everyone online right now (as of the start of this tick).
    pub fn player_list() -> Vec<Player> {
        let mut buf = vec![0u8; 1024];
        loop {
            let n = unsafe { players(buf.as_mut_ptr(), len_u8(&buf)) };
            let Ok(n) = usize::try_from(n) else {
                return Vec::new();
            };
            if n <= buf.len() {
                return serde_json::from_slice(&buf[..n]).unwrap_or_default();
            }
            buf.resize(n, 0);
        }
    }

    fn len_u8(b: &[u8]) -> i32 {
        i32::try_from(b.len()).unwrap_or(i32::MAX)
    }
}

#[doc(hidden)]
pub mod rt {
    use super::{Event, EventResult, Plugin};

    pub fn alloc(len: i32) -> *mut u8 {
        let buf = vec![0u8; usize::try_from(len).unwrap_or(0)].into_boxed_slice();
        Box::into_raw(buf) as *mut u8
    }

    pub unsafe fn handle<P: Plugin>(plugin: &mut P, ptr: *mut u8, len: i32) -> i32 {
        let len = usize::try_from(len).unwrap_or(0);
        let buf = unsafe { Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr, len)) };
        match serde_json::from_slice::<Event>(&buf) {
            Ok(event) => match plugin.on_event(&event) {
                EventResult::Cancel => 1,
                EventResult::Continue => 0,
            },
            Err(_) => 0,
        }
    }
}

#[macro_export]
macro_rules! export_plugin {
    ($ty:ty) => {
        static __BELUMI_PLUGIN: ::std::sync::Mutex<Option<$ty>> = ::std::sync::Mutex::new(None);

        #[unsafe(no_mangle)]
        pub extern "C" fn belumi_alloc(len: i32) -> i32 {
            $crate::rt::alloc(len) as usize as i32
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn belumi_init() {
            let mut p = <$ty as ::core::default::Default>::default();
            $crate::Plugin::init(&mut p);
            *__BELUMI_PLUGIN.lock().unwrap() = Some(p);
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn belumi_on_event(ptr: i32, len: i32) -> i32 {
            let mut guard = __BELUMI_PLUGIN.lock().unwrap();
            let p = guard.get_or_insert_with(<$ty as ::core::default::Default>::default);
            unsafe { $crate::rt::handle(p, ptr as usize as *mut u8, len) }
        }
    };
}
