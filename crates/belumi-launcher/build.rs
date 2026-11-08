//! Embeds Belumi's icon (`packaging/icons/belumi.ico`) in belumi-launcher.exe: Explorer, the task bar and the shortcuts show it.
fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let ico = format!("{dir}/../../packaging/icons/belumi.ico");
    println!("cargo:rerun-if-changed={ico}");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(&ico);
        if let Err(e) = resource.compile() {
            panic!("embedding {ico}: {e}");
        }
    }
}
