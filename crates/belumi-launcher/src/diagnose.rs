use crate::auth::Account;
use crate::game::{self, Target};
use crate::settings::Settings;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const BENCHMARK_LIMIT: Duration = Duration::from_secs(480);
const SCREENS_LIMIT: Duration = Duration::from_secs(150);
const SERVER_LIMIT: Duration = Duration::from_secs(120);

/// Wait for a child process, killing it after `limit`; the exit code, or none after a kill.
fn wait(child: &mut std::process::Child, limit: Duration) -> Option<i32> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code(),
            Ok(None) if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(250)),
            Err(_) => return None,
        }
    }
}

fn copy_packs(from: &Path, to: &Path) {
    let _ = std::fs::create_dir_all(to);
    for entry in std::fs::read_dir(from).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
            let _ = std::fs::copy(&path, to.join(entry.file_name()));
        }
    }
}

/// `Horizon: first frame after 700 ms` and `resources loaded in 2.1s` from a client log.
pub fn timings(log: &str) -> Value {
    let mut out = serde_json::Map::new();
    for line in log.lines() {
        if let Some(rest) = line.split_once("first frame after ").map(|(_, r)| r)
            && let Some(ms) = rest.split_whitespace().next().and_then(|n| n.parse::<f64>().ok())
        {
            out.insert("first_frame_ms".into(), json!(ms));
        }
        if let Some(rest) = line.split_once("resources loaded in ").map(|(_, r)| r)
            && let Some(s) = rest.split('s').next().and_then(|n| n.trim().parse::<f64>().ok())
        {
            out.insert("resources_loaded_s".into(), json!(s));
        }
    }
    Value::Object(out)
}

/// The numbers worth reading from a benchmark report: per scenario average fps and the 1 % low.
pub fn benchmark_summary(report: &Value) -> Value {
    let mut scenes = serde_json::Map::new();
    if let Some(metrics) = report["metrics"].as_object() {
        for (key, value) in metrics {
            let mut parts = key.split('.');
            let (Some("fps"), Some(scene), Some(what)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
            if what == "avg_fps" || what == "low_1pct_fps" {
                scenes
                    .entry(scene.to_owned())
                    .or_insert_with(|| json!({}))[what] = json!(value["mean"]);
            }
        }
    }
    json!({ "scenes": scenes, "memory_gpu_mb": report["metrics"]["memory.gpu_mb"]["mean"] })
}

struct Run {
    name: &'static str,
    shaders: bool,
}

fn client_run(settings: &Settings, dir: &Path, shaders: bool, args: &[String], limit: Duration, log_name: &str) -> (Option<i32>, f64, String) {
    let mut s = settings.clone();
    s.game_dir = dir.join("game").display().to_string();
    s.fullscreen = false;
    s.window_width = 0;
    s.window_height = 0;
    let log_path = dir.join(log_name);
    let log = std::fs::File::create(&log_path).ok();
    let mut env: Vec<(&str, &str)> = Vec::new();
    if !shaders {
        env.push(("HORIZON_SHADER_PACK", ""));
    }
    let started = Instant::now();
    let mut extra = vec!["--home".to_owned(), dir.join("home").display().to_string(), "--name".to_owned(), "Tester".to_owned()];
    extra.extend_from_slice(args);
    let code = match game::launch_with(&s, None, &Target::MainMenu, log, &extra, &env) {
        Ok(mut child) => wait(&mut child, limit),
        Err(e) => {
            let _ = std::fs::write(&log_path, e);
            None
        }
    };
    (code, started.elapsed().as_secs_f64(), crate::ui::tail(&log_path, 2_000_000))
}

fn system_notes() -> Value {
    let read = |p: &str| std::fs::read_to_string(p).unwrap_or_default();
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "launcher_version": env!("CARGO_PKG_VERSION"),
        "processor": std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| read("/proc/cpuinfo").lines().find(|l| l.starts_with("model name")).unwrap_or("").to_owned()),
        "language": crate::i18n::system_language(),
    })
}

/// The whole diagnose run: two benchmark and screenshot runs (shaders on and off) in a scratch game
/// folder, and, with an account and a server, a short visit. Writes `summary.json`, `summary.txt` and a ZIP.
pub fn run(settings: &Settings, account: Option<&Account>, server: Option<&str>, out: &Path, progress: &dyn Fn(&str)) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(out)?;
    let real_game = crate::worlds::game_dir(&settings.game_dir);
    let checks: Vec<Value> = crate::check::run(settings)
        .into_iter()
        .map(|c| json!({ "check": c.what, "ok": c.ok, "hint": if c.ok { String::new() } else { c.hint } }))
        .collect();
    let mut summary = json!({ "system": system_notes(), "checks": checks, "runs": {} });
    let runs = [Run { name: "shaders", shaders: true }, Run { name: "no-shaders", shaders: false }];
    for run in runs {
        let dir = out.join("runs").join(run.name);
        progress(&format!("Benchmark ({})...", run.name));
        if run.shaders {
            copy_packs(&real_game.join("shaderpacks"), &dir.join("game").join("shaderpacks"));
        }
        let (code, seconds, log) = client_run(
            settings,
            &dir,
            run.shaders,
            &["--resources".into(), settings.resources.clone(), "--benchmark".into(), "standard".into()],
            BENCHMARK_LIMIT,
            "benchmark.log",
        );
        let report = std::fs::read_dir(dir.join("game").join("benchmarks"))
            .into_iter()
            .flatten()
            .flatten()
            .find(|e| e.path().extension().is_some_and(|x| x == "json"))
            .and_then(|e| std::fs::read_to_string(e.path()).ok())
            .and_then(|t| serde_json::from_str::<Value>(&t).ok());
        if let Some(r) = &report {
            if summary["system"]["hardware"].is_null() {
                summary["system"]["hardware"] = r["hardware"].clone();
            }
            let _ = std::fs::write(dir.join("benchmark.json"), serde_json::to_string_pretty(r).unwrap_or_default());
        }
        let _ = std::fs::remove_dir_all(dir.join("game").join("saves"));
        let _ = std::fs::remove_dir_all(dir.join("game").join("benchmarks"));

        progress(&format!("Screenshots ({})...", run.name));
        let (screens_code, _, screens_log) = client_run(
            settings,
            &dir,
            run.shaders,
            &[
                "--resources".into(),
                settings.resources.clone(),
                "--world".into(),
                "BelumiDiagnose".into(),
                "--seed".into(),
                "7".into(),
                "--creative".into(),
                "--diagnose-shots".into(),
                dir.join("shots").display().to_string(),
                "--timeout-seconds".into(),
                SCREENS_LIMIT.as_secs().to_string(),
            ],
            SCREENS_LIMIT + Duration::from_secs(30),
            "screens.log",
        );
        let _ = std::fs::remove_dir_all(dir.join("game"));
        let shots = std::fs::read_dir(dir.join("shots")).map(|d| d.flatten().count()).unwrap_or(0);
        summary["runs"][run.name] = json!({
            "benchmark_exit": code,
            "benchmark_seconds": seconds,
            "benchmark": report.as_ref().map(benchmark_summary),
            "screens_exit": screens_code,
            "screenshots": shots,
            "timings": timings(&screens_log),
            "benchmark_timings": timings(&log),
        });
    }
    if let (Some(account), Some(server)) = (account, server) {
        progress(&format!("Visiting {server}..."));
        let dir = out.join("server");
        let mut s = settings.clone();
        s.game_dir = dir.join("game").display().to_string();
        let log_path = dir.join("visit.log");
        let _ = std::fs::create_dir_all(&dir);
        let extra = [
            "--home".to_owned(),
            dir.join("home").display().to_string(),
            "--diagnose-shots".to_owned(),
            dir.join("shots").display().to_string(),
            "--timeout-seconds".to_owned(),
            SERVER_LIMIT.as_secs().to_string(),
        ];
        let code = match game::launch_with(&s, Some(account), &Target::Address(server.to_owned()), std::fs::File::create(&log_path).ok(), &extra, &[]) {
            Ok(mut child) => wait(&mut child, SERVER_LIMIT + Duration::from_secs(30)),
            Err(_) => None,
        };
        let log = crate::ui::tail(&log_path, 2_000_000);
        let shots = std::fs::read_dir(dir.join("shots")).map(|d| d.flatten().count()).unwrap_or(0);
        let _ = std::fs::remove_dir_all(dir.join("game"));
        summary["server"] = json!({
            "address": server,
            "exit": code,
            "screenshots": shots,
            "joined": log.contains("logged in") || shots > 0,
            "voice_connected": log.contains("Voice chat connected") || log.contains("Voice chat: authenticated"),
            "disconnected": log.lines().find(|l| l.contains("disconnected:")).unwrap_or("").to_owned(),
            "timings": timings(&log),
        });
    }
    std::fs::write(out.join("summary.json"), serde_json::to_string_pretty(&summary).unwrap_or_default())?;
    std::fs::write(out.join("summary.txt"), text_summary(&summary))?;
    redact_folder(out);
    let zip = out.join("belumi-report.zip");
    crate::crash::pack_dir(out, &zip)?;
    Ok(zip)
}

fn redact_folder(dir: &Path) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            redact_folder(&path);
        } else if path.extension().is_some_and(|e| matches!(e.to_str(), Some("log" | "txt" | "json"))) {
            if let Ok(bytes) = std::fs::read(&path) {
                let _ = std::fs::write(&path, crate::crash::redact(bytes));
            }
        }
    }
}

pub fn text_summary(s: &Value) -> String {
    let mut t = String::from("Belumi diagnose report\n======================\n\n");
    let sys = &s["system"];
    let hw = &sys["hardware"];
    t.push_str(&format!(
        "System: {} {} ({})\nCPU: {} ({} threads), RAM {} MB\nGPU: {} / {} / backend {}\nDisplay: {}x{} @ {} Hz\nLauncher {}\n\n",
        hw["os"].as_str().unwrap_or(sys["os"].as_str().unwrap_or("?")),
        hw["os_version"].as_str().unwrap_or(""),
        sys["arch"].as_str().unwrap_or("?"),
        hw["cpu"].as_str().unwrap_or(sys["processor"].as_str().unwrap_or("?")),
        hw["cpu_threads"],
        hw["ram_mb"],
        hw["gpu"]["name"].as_str().unwrap_or("?"),
        hw["gpu"]["driver"].as_str().unwrap_or("?"),
        hw["gpu"]["backend"].as_str().unwrap_or("?"),
        hw["display"]["width"],
        hw["display"]["height"],
        hw["display"]["refresh_hz"],
        sys["launcher_version"].as_str().unwrap_or("?"),
    ));
    t.push_str("Checks:\n");
    for c in s["checks"].as_array().into_iter().flatten() {
        t.push_str(&format!("  [{}] {} {}\n", if c["ok"].as_bool() == Some(true) { "ok" } else { "FAILED" }, c["check"].as_str().unwrap_or(""), c["hint"].as_str().unwrap_or("")));
    }
    for name in ["shaders", "no-shaders"] {
        let r = &s["runs"][name];
        if r.is_null() {
            continue;
        }
        t.push_str(&format!("\nRun: {name}\n  benchmark exit {}, {:.0} s; screenshots {}; screens exit {}\n", r["benchmark_exit"], r["benchmark_seconds"].as_f64().unwrap_or(0.0), r["screenshots"], r["screens_exit"]));
        t.push_str(&format!("  start: first frame {} ms, resources {} s\n", r["timings"]["first_frame_ms"], r["timings"]["resources_loaded_s"]));
        if let Some(scenes) = r["benchmark"]["scenes"].as_object() {
            for (scene, v) in scenes {
                t.push_str(&format!("  {scene}: {:.0} fps average, {:.0} fps 1% low\n", v["avg_fps"].as_f64().unwrap_or(0.0), v["low_1pct_fps"].as_f64().unwrap_or(0.0)));
            }
        }
    }
    if !s["server"].is_null() {
        let v = &s["server"];
        t.push_str(&format!(
            "\nServer visit: {}\n  joined {}, voice connected {}, screenshots {}, exit {}\n  {}\n",
            v["address"].as_str().unwrap_or(""),
            v["joined"],
            v["voice_connected"],
            v["screenshots"],
            v["exit"],
            v["disconnected"].as_str().unwrap_or("")
        ));
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timings_come_out_of_the_client_log() {
        let log = "Horizon: resources loaded in 2.1s; 35723 block states\nHorizon: first frame after 700 ms\n";
        let t = timings(log);
        assert_eq!(t["first_frame_ms"], json!(700.0));
        assert_eq!(t["resources_loaded_s"], json!(2.1));
        assert_eq!(timings("nothing"), json!({}));
    }

    #[test]
    fn a_benchmark_report_reduces_to_fps_per_scene() {
        let report = json!({"metrics": {
            "fps.forest.avg_fps": {"mean": 120.0},
            "fps.forest.low_1pct_fps": {"mean": 80.0},
            "fps.forest.low_01pct_fps": {"mean": 40.0},
            "chunks.forest.load_ms": {"mean": 10.0},
            "memory.gpu_mb": {"mean": 500.0}
        }});
        let s = benchmark_summary(&report);
        assert_eq!(s["scenes"]["forest"]["avg_fps"], json!(120.0));
        assert_eq!(s["scenes"]["forest"]["low_1pct_fps"], json!(80.0));
        assert!(s["scenes"]["forest"].get("low_01pct_fps").is_none());
        assert_eq!(s["memory_gpu_mb"], json!(500.0));
    }

    #[test]
    fn the_text_summary_names_the_numbers_and_the_failed_checks() {
        let s = json!({
            "system": {"os": "linux", "arch": "x86_64", "launcher_version": "0.1.0", "hardware": {"cpu": "Test CPU", "gpu": {"name": "Test GPU", "backend": "vulkan"}}},
            "checks": [{"check": "Resources", "ok": false, "hint": "missing"}],
            "runs": {"shaders": {"benchmark_exit": 0, "benchmark_seconds": 100.0, "screenshots": 4, "screens_exit": 0,
                "timings": {"first_frame_ms": 700.0}, "benchmark": {"scenes": {"forest": {"avg_fps": 120.0, "low_1pct_fps": 80.0}}}}}
        });
        let text = text_summary(&s);
        assert!(text.contains("Test GPU") && text.contains("[FAILED] Resources missing"));
        assert!(text.contains("forest: 120 fps average, 80 fps 1% low"));
    }
}
