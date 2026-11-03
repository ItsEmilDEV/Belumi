use belumi_plugin_sdk::{Event, EventResult, Plugin, export_plugin, host};

#[derive(Default)]
struct Hello {
    joins: u32,
}

impl Plugin for Hello {
    fn init(&mut self) {
        host::info("hello plugin ready");
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::PlayerJoin { uuid, name } => {
                self.joins += 1;
                let _ = host::send_message_to(uuid, &format!("Welcome, {name}!"));
            }
            Event::PluginCommand { uuid, args, .. } => {
                let _ = host::send_message_to(
                    uuid,
                    &format!(
                        "Hello from a Belumi plugin! args={args:?} joins={}",
                        self.joins
                    ),
                );
            }
            Event::Chat { message, .. } if message.to_lowercase().contains("badword") => {
                return EventResult::Cancel;
            }
            _ => {}
        }
        EventResult::Continue
    }
}

export_plugin!(Hello);
