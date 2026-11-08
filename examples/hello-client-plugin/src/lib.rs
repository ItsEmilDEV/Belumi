use belumi_client_plugin_sdk::{Event, EventResult, Plugin, Screen, export_plugin, host};

#[derive(Default)]
struct Hello {
    ticks: u64,
    presses: u32,
    loud: bool,
}

impl Hello {
    fn screen(&self) -> Screen {
        Screen::new("Hello")
            .label("A screen made by a plugin")
            .button("greet", "Say hello in chat")
            .toggle("loud", "Shout", self.loud)
    }
}

impl Plugin for Hello {
    fn init(&mut self) {
        host::info("hello-client-plugin ready");
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Tick { tick } => {
                self.ticks = *tick;
                if tick % 20 == 0 {
                    let line = match host::current_player() {
                        Some(p) => format!("Plugin: {} ticks, y {:.0}", self.ticks, p.y),
                        None => format!("Plugin: {} ticks", self.ticks),
                    };
                    let _ = host::set_hud("ticks", &line);
                }
            }
            Event::Key { key, pressed: true } if key == "key.keyboard.o" => {
                let _ = host::set_screen(&self.screen());
                let _ = host::open_screen();
            }
            Event::Key { pressed: true, .. } => {
                self.presses += 1;
                let _ = host::set_hud("keys", &format!("Plugin: {} key presses", self.presses));
            }
            Event::Command { name, args } if name == "hello" && args.trim() == "screen" => {
                let _ = host::set_screen(&self.screen());
                let _ = host::open_screen();
                return EventResult::Cancel;
            }
            Event::Screen { id, value } => {
                if id == "loud" {
                    self.loud = *value;
                    let _ = host::set_screen(&self.screen());
                } else if id == "greet" {
                    let text = if self.loud { "HELLO FROM THE SCREEN!" } else { "Hello from the screen." };
                    let _ = host::print_chat(text);
                }
            }
            Event::Command { name, args } if name == "hello" => {
                let who = if args.trim().is_empty() { "world" } else { args.trim() };
                let _ = host::print_chat(&format!("Hello, {who}!"));
                return EventResult::Cancel;
            }
            _ => {}
        }
        EventResult::Continue
    }
}

export_plugin!(Hello);
