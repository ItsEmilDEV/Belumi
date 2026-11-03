use belumi_plugin_api::{Event, EventResult, PluginManifest};
use belumi_server::PluginManager;

fn manifest(name: &str, extra: &str) -> PluginManifest {
    PluginManifest::parse(&format!(
        "name='{name}'\nversion='1'\napi_version=1\n{extra}"
    ))
    .unwrap()
}

const CANCEL: &str = r#"(module
  (import "belumi" "log" (func $log (param i32 i32 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "hi")
  (func (export "belumi_init") (call $log (i32.const 1) (i32.const 0) (i32.const 2)))
  (func (export "belumi_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "belumi_on_event") (param i32 i32) (result i32) (i32.const 1)))"#;

const LOOP: &str = r#"(module
  (memory (export "memory") 1)
  (func (export "belumi_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "belumi_on_event") (param i32 i32) (result i32) (loop $l (br $l)) (i32.const 0)))"#;

fn chat() -> Event {
    Event::Chat {
        uuid: "u".into(),
        message: "x".into(),
    }
}

#[test]
fn cancel_and_subscription_filter() {
    let mut m = PluginManager::new().unwrap();
    m.load_bytes(
        manifest("c", "subscribe=['chat']"),
        &wat::parse_str(CANCEL).unwrap(),
    )
    .unwrap();
    assert_eq!(m.emit(&chat()), EventResult::Cancel);
    assert_eq!(
        m.emit(&Event::PlayerJoin {
            uuid: "u".into(),
            name: "n".into()
        }),
        EventResult::Continue
    );
    let mut m2 = PluginManager::new().unwrap();
    m2.load_bytes(
        manifest("c", "subscribe=['tick']"),
        &wat::parse_str(CANCEL).unwrap(),
    )
    .unwrap();
    assert_eq!(m2.emit(&Event::Tick { tick: 1 }), EventResult::Continue);
}

#[test]
fn runaway_plugin_is_disabled_not_fatal() {
    let mut m = PluginManager::new().unwrap();
    m.load_bytes(
        manifest("loop", "subscribe=['chat']"),
        &wat::parse_str(LOOP).unwrap(),
    )
    .unwrap();
    m.load_bytes(
        manifest("c", "subscribe=['chat']"),
        &wat::parse_str(CANCEL).unwrap(),
    )
    .unwrap();
    assert_eq!(m.emit(&chat()), EventResult::Cancel);
    assert_eq!(m.emit(&chat()), EventResult::Cancel);
}

#[test]
fn load_dir_orders_by_depends_and_skips_broken() {
    let dir = tempfile::tempdir().unwrap();
    let wasm = wat::parse_str(CANCEL).unwrap();
    for (n, extra) in [("b", "depends=['a']"), ("a", "")] {
        let p = dir.path().join(n);
        std::fs::create_dir(&p).unwrap();
        std::fs::write(
            p.join("plugin.toml"),
            format!("name='{n}'\nversion='1'\napi_version=1\n{extra}"),
        )
        .unwrap();
        std::fs::write(p.join("plugin.wasm"), &wasm).unwrap();
    }
    let bad = dir.path().join("bad");
    std::fs::create_dir(&bad).unwrap();
    std::fs::write(
        bad.join("plugin.toml"),
        "name='bad'\nversion='1'\napi_version=1\n",
    )
    .unwrap();
    let mut m = PluginManager::new().unwrap();
    assert_eq!(m.load_dir(dir.path()).unwrap(), 2);
    assert_eq!(m.plugin_names(), vec!["a", "b"]);
}

const ACTIONS: &str = r#"(module
  (import "belumi" "broadcast" (func $b (param i32 i32) (result i32)))
  (import "belumi" "kick" (func $k (param i32 i32 i32 i32) (result i32)))
  (import "belumi" "player_count" (func $pc (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "hiur")
  (func (export "belumi_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "belumi_on_event") (param i32 i32) (result i32)
    (drop (call $b (i32.const 0) (i32.const 2)))
    (drop (call $k (i32.const 2) (i32.const 1) (i32.const 3) (i32.const 1)))
    ;; cancel iff player_count == 7
    (i32.eq (call $pc) (i32.const 7))))"#;

#[test]
fn host_calls_respect_permissions() {
    use belumi_server::Action;
    let mut m = PluginManager::new().unwrap();
    m.load_bytes(
        manifest("a", "subscribe=['chat']\npermissions=['messaging']"),
        &wat::parse_str(ACTIONS).unwrap(),
    )
    .unwrap();
    assert!(m.sync(7).is_empty());
    assert_eq!(
        m.emit(&chat()),
        EventResult::Cancel,
        "player_count reads what sync published"
    );
    assert_eq!(m.sync(7), vec![Action::Broadcast("hi".into())]);
    assert!(m.sync(7).is_empty());
}

#[test]
fn plugin_commands_go_to_their_owner_only() {
    let mut m = PluginManager::new().unwrap();
    m.load_bytes(
        manifest("a", "commands=['hello']\npermissions=['messaging']"),
        &wat::parse_str(ACTIONS).unwrap(),
    )
    .unwrap();
    assert_eq!(m.command_names(), vec!["hello"]);
    assert!(!m.try_command("u", "/other"));
    assert!(m.sync(0).is_empty());
    assert!(m.try_command("u", "/Hello world"));
    assert_eq!(
        m.sync(0),
        vec![belumi_server::Action::Broadcast("hi".into())],
        "the command handler ran"
    );
}
