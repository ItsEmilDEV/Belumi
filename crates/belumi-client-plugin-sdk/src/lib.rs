#![allow(unsafe_code)]

pub use belumi_client_plugin_api::{ABI_VERSION, Event, EventResult};

/// A screen built row by row: `Screen::new("Title").label("Hello").button("go", "Go").toggle("t", "Option", true)`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Screen(belumi_client_plugin_api::ScreenSpec);

impl Screen {
    pub fn new(title: &str) -> Self {
        Self(belumi_client_plugin_api::ScreenSpec { title: title.to_owned(), rows: Vec::new() })
    }
    pub fn label(mut self, text: &str) -> Self {
        self.0.rows.push(belumi_client_plugin_api::ScreenRow::Label { text: text.to_owned() });
        self
    }
    pub fn button(mut self, id: &str, text: &str) -> Self {
        self.0.rows.push(belumi_client_plugin_api::ScreenRow::Button { id: id.to_owned(), text: text.to_owned() });
        self
    }
    pub fn toggle(mut self, id: &str, text: &str, on: bool) -> Self {
        self.0.rows.push(belumi_client_plugin_api::ScreenRow::Toggle { id: id.to_owned(), text: text.to_owned(), on });
        self
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.0).unwrap_or_default()
    }
}

pub trait Plugin: Default {
    fn init(&mut self) {}
    fn on_event(&mut self, event: &Event) -> EventResult;
}

pub mod host {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HostError {
        Denied,
        Full,
        Invalid,
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "belumi")]
    unsafe extern "C" {
        fn log(level: i32, ptr: *const u8, len: i32);
        fn chat_print(ptr: *const u8, len: i32) -> i32;
        fn hud_set(ip: *const u8, il: i32, tp: *const u8, tl: i32) -> i32;
        fn player(ptr: *mut u8, cap: i32) -> i32;
        fn screen_set(ptr: *const u8, len: i32) -> i32;
        fn screen_open() -> i32;
        fn screen_close() -> i32;
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod stub {
        pub unsafe fn log(_: i32, _: *const u8, _: i32) {}
        pub unsafe fn chat_print(_: *const u8, _: i32) -> i32 {
            0
        }
        pub unsafe fn hud_set(_: *const u8, _: i32, _: *const u8, _: i32) -> i32 {
            0
        }
        pub unsafe fn player(_: *mut u8, _: i32) -> i32 {
            2
        }
        pub unsafe fn screen_set(_: *const u8, _: i32) -> i32 {
            0
        }
        pub unsafe fn screen_open() -> i32 {
            0
        }
        pub unsafe fn screen_close() -> i32 {
            0
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    use stub::*;

    fn result(code: i32) -> Result<(), HostError> {
        match code {
            0 => Ok(()),
            1 => Err(HostError::Denied),
            2 => Err(HostError::Full),
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

    pub fn print_chat(text: &str) -> Result<(), HostError> {
        result(unsafe { chat_print(text.as_ptr(), len(text)) })
    }

    pub fn set_hud(id: &str, text: &str) -> Result<(), HostError> {
        result(unsafe { hud_set(id.as_ptr(), len(id), text.as_ptr(), len(text)) })
    }

    pub fn clear_hud(id: &str) -> Result<(), HostError> {
        set_hud(id, "")
    }

    /// Sets the plugin's screen (needs the `screen` permission). It shows on the client screen's Plugins page and opens
    /// with `open_screen`; presses arrive as `Event::Screen`.
    pub fn set_screen(screen: &super::Screen) -> Result<(), HostError> {
        let json = screen.to_json();
        result(unsafe { screen_set(json.as_ptr(), len(&json)) })
    }

    pub fn open_screen() -> Result<(), HostError> {
        result(unsafe { screen_open() })
    }

    pub fn close_screen() -> Result<(), HostError> {
        result(unsafe { screen_close() })
    }

    #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
    pub struct Player {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub dimension: String,
        pub fps: u32,
    }

    pub fn current_player() -> Option<Player> {
        let mut buf = vec![0u8; 256];
        loop {
            let n = unsafe { player(buf.as_mut_ptr(), i32::try_from(buf.len()).ok()?) };
            let n = usize::try_from(n).ok()?;
            if n <= buf.len() {
                return serde_json::from_slice(&buf[..n]).ok();
            }
            buf.resize(n, 0);
        }
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
