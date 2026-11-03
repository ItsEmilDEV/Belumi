use belumi_server::config::{BelumiConfig, TEMPLATE};
use std::{path::PathBuf, process::Command};

fn fail(msg: &str) -> ! {
    eprintln!("belumi-server: {msg}");
    std::process::exit(2);
}

fn main() {
    let (mut dir, mut client, mut resources) =
        (PathBuf::from("."), None::<PathBuf>, None::<PathBuf>);
    let mut extra = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .unwrap_or_else(|| fail(&format!("{name} needs a value")))
        };
        match a.as_str() {
            "--dir" => dir = value("--dir").into(),
            "--client" => client = Some(value("--client").into()),
            "--resources" => resources = Some(value("--resources").into()),
            "--" => {
                extra.extend(args.by_ref());
            }
            "-h" | "--help" => {
                println!(
                    "usage: belumi-server [--dir DIR] [--client PATH] [--resources PATH] [-- EXTRA ARGS]\nsettings: DIR/belumi.toml"
                );
                return;
            }
            other => fail(&format!("unknown argument {other:?} (see --help)")),
        }
    }
    std::fs::create_dir_all(dir.join("plugins"))
        .unwrap_or_else(|e| fail(&format!("{}: {e}", dir.display())));
    let config_path = dir.join("belumi.toml");
    if !config_path.exists() {
        std::fs::write(&config_path, TEMPLATE)
            .unwrap_or_else(|e| fail(&format!("cannot write belumi.toml: {e}")));
        eprintln!("belumi-server: wrote {}", config_path.display());
    }
    let config = BelumiConfig::parse(&std::fs::read_to_string(&config_path).unwrap_or_default())
        .unwrap_or_else(|e| fail(&format!("{}: {e}", config_path.display())));

    let client = client
        .or(config.server.client.clone())
        .unwrap_or_else(|| "horizon-client".into());
    let resources = resources.or(config.server.resources.clone());
    let mut cmd = Command::new(&client);
    cmd.current_dir(&dir);
    if let Some(r) = resources {
        cmd.arg("--resources")
            .arg(std::path::absolute(r).unwrap_or_else(|e| fail(&format!("resources: {e}"))));
    }
    cmd.arg("--dedicated").arg("--nogui");
    cmd.args(extra);
    for (k, v) in config.env() {
        cmd.env(k, v);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let e = cmd.exec();
        fail(&format!("cannot start {}: {e}", client.display()));
    }
    #[cfg(not(unix))]
    {
        let status = cmd
            .status()
            .unwrap_or_else(|e| fail(&format!("cannot start {}: {e}", client.display())));
        std::process::exit(status.code().unwrap_or(1));
    }
}
