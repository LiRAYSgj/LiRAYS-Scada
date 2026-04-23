use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use super::config::{load_settings, resolve_data_dir};
use crate::settings::settings::{SettingSpec, Settings};

pub(super) async fn watch_metrics(config_arg: Option<PathBuf>) -> Result<(), String> {
    let (settings, used_path) = load_settings(config_arg)?;
    ensure_real_time_metrics_enabled(&settings)?;
    let metrics_dir = resolve_data_dir(&settings)?.join("metrics");
    let rt_path = metrics_dir.join("metrics_rt.txt");

    if let Some(path) = used_path {
        println!("Settings source: {}", path.display());
    }
    println!("Watching metrics snapshot: {}", rt_path.display());
    println!("Press Ctrl+C to stop.");

    loop {
        let rendered = render_metrics_snapshot(&rt_path).await;
        print!("\x1B[2J\x1B[H");
        println!("LiRAYS metrics snapshot");
        println!("Path: {}", rt_path.display());
        println!("Refresh: 1s");
        println!("Press Ctrl+C to stop");
        println!();
        println!("{rendered}");
        io::stdout()
            .flush()
            .map_err(|e| format!("Failed to flush terminal output: {e}"))?;

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!();
                println!("Stopped metrics watch.");
                return Ok(());
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
}

fn ensure_real_time_metrics_enabled(settings: &Settings) -> Result<(), String> {
    let enabled: bool = settings
        .value(&SettingSpec {
            section: "metrics",
            key: "real_time",
            env_var: "METRICS_REAL_TIME",
            default: false,
        })
        .map_err(|e| format!("Failed to read metrics.real_time: {e}"))?;

    if enabled {
        Ok(())
    } else {
        Err("Real-time metrics are disabled. Set metrics.real_time=true (or METRICS_REAL_TIME=true), restart the service, then run `lirays watch-metrics`.".to_string())
    }
}

async fn render_metrics_snapshot(path: &Path) -> String {
    match tokio::fs::read_to_string(path).await {
        Ok(content) if content.trim().is_empty() => {
            "metrics_rt.txt is currently empty. Waiting for new metrics...".to_string()
        }
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => format!(
            "Waiting for metrics output...\nFile not found yet: {}\nConfirm the service is running and metrics are enabled.",
            path.display()
        ),
        Err(err) => format!("Failed to read {}: {err}", path.display()),
    }
}
