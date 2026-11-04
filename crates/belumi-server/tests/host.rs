use belumi_plugin_api::{Event, EventResult, PluginManifest};
use belumi_server::{CommandOutcome, PluginManager};

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
    m.set_roster(&roster(7));
    assert!(m.sync().is_empty());
    assert_eq!(
        m.emit(&chat()),
        EventResult::Cancel,
        "player_count reads the roster"
    );
    assert_eq!(m.sync(), vec![Action::Broadcast("hi".into())]);
    assert!(m.sync().is_empty());
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
    assert_eq!(m.try_command("u", 0, "/other"), CommandOutcome::NotPlugin);
    assert!(m.sync().is_empty());
    assert_eq!(m.try_command("u", 0, "/Hello world"), CommandOutcome::Ran);
    assert_eq!(
        m.sync(),
        vec![belumi_server::Action::Broadcast("hi".into())],
        "the command handler ran"
    );
}

fn roster(n: usize) -> Vec<(String, String)> {
    (0..n).map(|i| (format!("u{i}"), format!("P{i}"))).collect()
}

#[test]
fn command_levels_gate_plugin_commands() {
    let mut m = PluginManager::new().unwrap();
    m.load_bytes(
        manifest(
            "a",
            "commands=['open','admin']\npermissions=['messaging']\n[command_levels]\nadmin=3",
        ),
        &wat::parse_str(ACTIONS).unwrap(),
    )
    .unwrap();
    assert_eq!(m.try_command("u", 0, "/open"), CommandOutcome::Ran);
    assert_eq!(m.sync().len(), 1, "the plugin broadcast once");
    assert_eq!(m.try_command("u", 2, "/admin"), CommandOutcome::Denied);
    assert!(
        matches!(&m.sync()[..], [belumi_server::Action::Message { uuid, .. }] if uuid == "u"),
        "only the player was told; the plugin never ran"
    );
    assert_eq!(m.try_command("u", 3, "/admin"), CommandOutcome::Ran);
}

/// `players` fills a buffer with the roster JSON and returns its length; a
/// too small buffer gets nothing written and the needed size back.
const ROSTER: &str = r#"(module
  (import "belumi" "players" (func $players (param i32 i32) (result i32)))
  (import "belumi" "broadcast" (func $broadcast (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "belumi_alloc") (param i32) (result i32) (i32.const 0))
  (func (export "belumi_on_event") (param i32 i32) (result i32)
    (local $n i32)
    (local.set $n (call $players (i32.const 1024) (i32.const 4096)))
    (drop (call $broadcast (i32.const 1024) (local.get $n)))
    (i32.const 0)))"#;

#[test]
fn players_import_lists_the_roster() {
    let mut m = PluginManager::new().unwrap();
    m.load_bytes(
        manifest("a", "subscribe=['chat']\npermissions=['messaging']"),
        &wat::parse_str(ROSTER).unwrap(),
    )
    .unwrap();
    m.set_roster(&roster(2));
    m.emit(&chat());
    match &m.sync()[..] {
        [belumi_server::Action::Broadcast(json)] => {
            let v: serde_json::Value = serde_json::from_str(json).unwrap();
            assert_eq!(v[1]["name"], "P1");
            assert_eq!(v[0]["uuid"], "u0");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn reload_picks_up_changed_plugins() {
    let dir = tempfile::tempdir().unwrap();
    let write = |wat: &str| {
        let f = dir.path().join("p");
        std::fs::create_dir_all(&f).unwrap();
        std::fs::write(
            f.join("plugin.toml"),
            "name='p'\nversion='1'\napi_version=1\ncommands=['x']\npermissions=['messaging']\n",
        )
        .unwrap();
        std::fs::write(f.join("plugin.wasm"), wat::parse_str(wat).unwrap()).unwrap();
    };
    write(ACTIONS);
    let mut m = PluginManager::new().unwrap();
    assert_eq!(m.load_dir(dir.path()).unwrap(), 1);
    m.try_command("u", 0, "/x");
    assert_eq!(m.sync().len(), 1);
    write(ROSTER);
    assert_eq!(m.reload().unwrap(), 1);
    m.set_roster(&roster(1));
    m.try_command("u", 0, "/x");
    assert_eq!(m.sync().len(), 1, "the new plugin answers");
}
