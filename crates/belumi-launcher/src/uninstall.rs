//! Uninstall: what to delete for "only the cache and usage data", "only the app" and "everything". The plan is worked out first
//! (and shown to the user), then run. Nothing outside Belumi's own folders and files is ever on a plan.
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Caches and what the game recorded about your use (chunk cache, maps, heads, logs, statistics); settings and worlds stay.
    Cache,
    /// The program files and menu entries; settings, accounts and worlds stay.
    App,
    /// The app and all its data (settings, accounts, resources, caches), and on request the worlds and screenshots too.
    Everything,
}

#[derive(Clone, Debug)]
pub struct Context {
    /// The folder the launcher runs from (next to `horizon-client`).
    pub exe_dir: Option<PathBuf>,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    /// The game folder from the settings, when one is set.
    pub game_dir: Option<PathBuf>,
    pub home: PathBuf,
    /// Also the worlds and screenshots in the game folder (only for `Everything`).
    pub worlds: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub paths: Vec<PathBuf>,
    /// On Windows the installer's own uninstaller removes the program files; it runs last.
    pub uninstaller: Option<PathBuf>,
    /// Why the program files cannot be removed (a development build), if so.
    pub refused: Option<String>,
}

/// What the game keeps in its folder that is only cache or a record of use.
const GAME_CACHE: [&str; 7] =
    ["belumi-chunks", "belumi-maps", "belumi-history", "belumi-pats.json", "belumi-combat.json", "belumi-chests.json", "belumi-rollback.json"];

fn horizon_dir(ctx: &Context) -> Option<PathBuf> {
    (ctx.data_dir.file_name()? == "belumi").then(|| ctx.data_dir.parent().map(|p| p.join("horizon")))?
}

/// A path is only deleted when it is absolute, deep enough and not the home folder or above.
pub fn safe(path: &Path, home: &Path) -> bool {
    path.is_absolute()
        && path != home
        && !home.starts_with(path)
        && path.components().filter(|c| matches!(c, Component::Normal(_))).count() >= 3
        && !path.components().any(|c| c == Component::ParentDir)
}

fn app_files(ctx: &Context, plan: &mut Plan) {
    let Some(dir) = &ctx.exe_dir else { return };
    if dir.components().any(|c| c.as_os_str() == "target") {
        plan.refused = Some("This is a development build (a target folder); nothing is removed.".into());
        return;
    }
    if cfg!(windows) {
        let unins = dir.join("unins000.exe");
        if unins.is_file() {
            plan.uninstaller = Some(unins);
            return;
        }
    }
    let suffix = std::env::consts::EXE_SUFFIX;
    for base in ["belumi-launcher", "horizon-client"] {
        plan.paths.push(dir.join(format!("{base}{suffix}")));
        plan.paths.push(dir.join(format!("{base}{suffix}.old")));
    }
    if dir.starts_with(&ctx.home) {
        let share = ctx.home.join(".local/share");
        plan.paths.push(share.join("applications/belumi-launcher.desktop"));
        plan.paths.push(share.join("applications/belumi.desktop"));
        plan.paths.push(share.join("icons/hicolor/512x512/apps/belumi.png"));
    }
}

fn cache_files(ctx: &Context, plan: &mut Plan) {
    plan.paths.push(ctx.config_dir.join("heads"));
    plan.paths.push(ctx.data_dir.join("logs"));
    plan.paths.push(ctx.data_dir.join("belumi-report"));
    if let Some(h) = horizon_dir(ctx) {
        plan.paths.push(h.join("logs"));
        plan.paths.push(h.join("crash-reports"));
    }
    if let Some(g) = &ctx.game_dir {
        plan.paths.extend(GAME_CACHE.iter().map(|n| g.join(n)));
    }
}

/// What a scope removes, as far as it exists. The list is what the confirmation shows.
pub fn plan(scope: Scope, ctx: &Context) -> Plan {
    let mut plan = Plan::default();
    match scope {
        Scope::Cache => cache_files(ctx, &mut plan),
        Scope::App => app_files(ctx, &mut plan),
        Scope::Everything => {
            app_files(ctx, &mut plan);
            plan.paths.push(ctx.config_dir.clone());
            plan.paths.push(ctx.data_dir.clone());
            if let Some(h) = horizon_dir(ctx) {
                plan.paths.push(h);
            }
            if let Some(g) = &ctx.game_dir {
                if let Ok(entries) = std::fs::read_dir(g) {
                    plan.paths.extend(entries.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("belumi-")).map(|e| e.path()));
                }
                if ctx.worlds {
                    plan.paths.extend(["saves", "screenshots"].iter().map(|n| g.join(n)));
                }
            }
        }
    }
    plan.paths.retain(|p| safe(p, &ctx.home) && p.symlink_metadata().is_ok());
    plan.paths.sort();
    plan.paths.dedup();
    plan
}

#[derive(Debug, Default)]
pub struct Report {
    pub removed: usize,
    pub failed: Vec<(PathBuf, String)>,
}

/// Deletes everything on the plan; a path that fails is reported and the rest goes on.
pub fn run(plan: &Plan, home: &Path) -> Report {
    let mut report = Report::default();
    for path in &plan.paths {
        if !safe(path, home) {
            report.failed.push((path.clone(), "not a Belumi path".into()));
            continue;
        }
        let result = match path.symlink_metadata() {
            Ok(m) if m.is_dir() => std::fs::remove_dir_all(path),
            Ok(_) => std::fs::remove_file(path),
            Err(_) => continue,
        };
        match result {
            Ok(()) => report.removed += 1,
            Err(e) => report.failed.push((path.clone(), e.to_string())),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(root: &Path) -> Context {
        let home = root.join("home/me");
        let ctx = Context {
            exe_dir: Some(home.join(".local/bin")),
            config_dir: home.join(".config/belumi"),
            data_dir: home.join(".local/share/belumi"),
            game_dir: Some(home.join("games/belumi")),
            home: home.clone(),
            worlds: false,
        };
        for d in [
            ".local/bin", ".local/share/applications", ".config/belumi/heads", ".local/share/belumi/logs", ".local/share/belumi/resources",
            ".local/share/horizon/logs", "games/belumi/saves/w1", "games/belumi/belumi-chunks", "games/belumi/belumi-plugins",
        ] {
            std::fs::create_dir_all(home.join(d)).unwrap();
        }
        for f in [
            ".local/bin/belumi-launcher", ".local/bin/horizon-client", ".local/share/applications/belumi.desktop",
            ".config/belumi/launcher.json", "games/belumi/belumi-pats.json", "games/belumi/belumi-patpat.json", "games/belumi/options.txt",
        ] {
            std::fs::write(home.join(f), "x").unwrap();
        }
        ctx
    }

    fn names(plan: &Plan, ctx: &Context) -> Vec<String> {
        plan.paths.iter().map(|p| p.strip_prefix(&ctx.home).unwrap().to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn the_cache_scope_keeps_settings_worlds_and_the_program() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = context(dir.path());
        let list = names(&plan(Scope::Cache, &ctx), &ctx);
        assert!(list.contains(&".config/belumi/heads".to_string()) && list.contains(&"games/belumi/belumi-chunks".to_string()));
        assert!(list.contains(&"games/belumi/belumi-pats.json".to_string()) && list.contains(&".local/share/horizon/logs".to_string()));
        assert!(!list.iter().any(|p| p.contains("launcher.json") || p.contains("saves") || p.contains("bin") || p.contains("patpat")));
    }

    #[test]
    fn the_app_scope_removes_only_program_files_and_menu_entries() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = context(dir.path());
        let list = names(&plan(Scope::App, &ctx), &ctx);
        assert_eq!(list, [".local/bin/belumi-launcher", ".local/bin/horizon-client", ".local/share/applications/belumi.desktop"]);
    }

    #[test]
    fn everything_takes_the_data_but_the_worlds_only_on_request() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = context(dir.path());
        let list = names(&plan(Scope::Everything, &ctx), &ctx);
        for want in [".config/belumi", ".local/share/belumi", ".local/share/horizon", "games/belumi/belumi-plugins", "games/belumi/belumi-patpat.json"] {
            assert!(list.contains(&want.to_string()), "{want} in {list:?}");
        }
        assert!(!list.iter().any(|p| p.contains("saves") || p.contains("options.txt")));
        ctx.worlds = true;
        assert!(names(&plan(Scope::Everything, &ctx), &ctx).contains(&"games/belumi/saves".to_string()));
    }

    #[test]
    fn running_a_plan_deletes_it_and_reports_what_it_could_not() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = context(dir.path());
        let p = plan(Scope::Cache, &ctx);
        let report = run(&p, &ctx.home);
        assert!(report.failed.is_empty() && report.removed == p.paths.len() && report.removed >= 5);
        assert!(!ctx.config_dir.join("heads").exists() && ctx.config_dir.join("launcher.json").exists());
        assert!(ctx.game_dir.unwrap().join("saves/w1").exists());
        assert_eq!(run(&p, &ctx.home).removed, 0, "a second run finds nothing");
    }

    #[test]
    fn a_development_build_and_dangerous_paths_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = context(dir.path());
        ctx.exe_dir = Some(ctx.home.join("src/belumi/target/release"));
        let p = plan(Scope::App, &ctx);
        assert!(p.refused.is_some() && p.paths.is_empty());
        assert!(!safe(&ctx.home, &ctx.home) && !safe(Path::new("/"), &ctx.home) && !safe(Path::new("/home"), &ctx.home));
        assert!(!safe(Path::new("relative/dir/x"), &ctx.home) && !safe(&ctx.home.join("a/../../b"), &ctx.home));
        assert!(safe(&ctx.home.join(".config/belumi"), &ctx.home));
    }
}
