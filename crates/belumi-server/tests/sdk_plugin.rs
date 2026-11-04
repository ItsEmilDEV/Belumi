use belumi_plugin_api::{Event, EventResult, PluginManifest};
use belumi_server::{Action, PluginManager};

#[test]
fn hello_plugin_built_with_the_sdk() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm_path = root.join("target/wasm32-unknown-unknown/release/hello_plugin.wasm");
    let Ok(wasm) = std::fs::read(&wasm_path) else {
        eprintln!("skipped: {} not built", wasm_path.display());
        return;
    };
    let manifest = PluginManifest::parse(
        &std::fs::read_to_string(root.join("examples/hello-plugin/plugin.toml")).unwrap(),
    )
    .unwrap();
    let mut m = PluginManager::new().unwrap();
    m.load_bytes(manifest, &wasm).unwrap();

    assert!(m.sync().is_empty());
    m.emit(&Event::PlayerJoin {
        uuid: "u-1".into(),
        name: "Alex".into(),
    });
    assert_eq!(
        m.sync(),
        vec![Action::Message {
            uuid: "u-1".into(),
            text: "Welcome, Alex!".into()
        }]
    );

    assert_eq!(m.try_command("u-1", 0, "/hello a b"), belumi_server::CommandOutcome::Ran);
    let acts = m.sync();
    assert_eq!(acts.len(), 1);
    assert!(
        matches!(&acts[0], Action::Message { text, .. } if text == "Hello from a Belumi plugin! args=\"a b\" joins=1"),
        "{acts:?}"
    );

    m.set_roster(&[("u-1".into(), "Alex".into()), ("u-2".into(), "Sam".into())]);
    assert_eq!(
        m.try_command("u-1", 0, "/players"),
        belumi_server::CommandOutcome::Ran
    );
    assert_eq!(
        m.sync(),
        vec![Action::Message {
            uuid: "u-1".into(),
            text: "2 online: Alex, Sam".into()
        }]
    );
    assert_eq!(
        m.try_command("u-1", 1, "/shout hi"),
        belumi_server::CommandOutcome::Denied
    );
    m.sync();
    assert_eq!(
        m.try_command("u-1", 2, "/shout hi there"),
        belumi_server::CommandOutcome::Ran
    );
    assert_eq!(m.sync(), vec![Action::Broadcast("HI THERE".into())]);

    let chat = |t: &str| Event::Chat {
        uuid: "u-1".into(),
        message: t.into(),
    };
    assert_eq!(m.emit(&chat("hi")), EventResult::Continue);
    assert_eq!(m.emit(&chat("a BadWord here")), EventResult::Cancel);
}
