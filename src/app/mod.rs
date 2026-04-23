use std::fs;

use log::info;

use crate::http::run_http_server;
use crate::settings::{load_runtime_config, parse_config_arg};

pub async fn run() {
    let config_path = parse_config_arg();
    let config = load_runtime_config(config_path);
    let rt_db_dir = config.data_dir.join("rt_data");
    let sessions_dir = config.data_dir.join("sessions");
    let static_db_file = config.data_dir.join("static.db");
    env_logger::Builder::new()
        .filter_level(config.log_level)
        .filter_module("sqlx::query", log::LevelFilter::Warn)
        .filter_module("sqlx", log::LevelFilter::Warn)
        .filter_module("sled", log::LevelFilter::Warn)
        .target(env_logger::Target::Stdout)
        .init();
    if let Err(e) = fs::create_dir_all(&rt_db_dir) {
        eprintln!("Failed to create data dir {}: {e}", rt_db_dir.display());
        std::process::exit(1);
    }
    if let Err(e) = fs::create_dir_all(&sessions_dir) {
        eprintln!(
            "Failed to create sessions dir {}: {e}",
            sessions_dir.display()
        );
        std::process::exit(1);
    }

    let http_tls = config.server_tls.clone();
    let http_schema = if http_tls.is_some() { "https" } else { "http" };
    info!(
        "Starting server on {http_schema}://{}:{}",
        config.host, config.port
    );

    let metrics_dir = config.data_dir.join("metrics");

    if let Err(e) = run_http_server(
        &config.host,
        config.port,
        &rt_db_dir,
        &sessions_dir,
        &static_db_file,
        http_tls,
        metrics_dir,
        config.metrics_real_time,
        config.metrics_historic,
        config.flush_ms,
        config.auth_enabled,
        config.auth_access_ttl,
        config.auth_refresh_ttl,
        config.auth_secret_bytes.clone(),
    )
    .await
    {
        eprintln!("Server failed to start: {e}");
        std::process::exit(1);
    }
}
