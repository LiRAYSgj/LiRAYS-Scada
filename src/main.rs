mod http;
mod migration;
mod rtdata;
mod settings;
mod tls;

use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

pub(crate) const DEFAULT_SETTINGS_YAML: &str =
    include_str!("../packaging/debian/deb-files/etc/lirays-scada/settings.yaml");

use log::info;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use tokio::sync::oneshot;

use http::run_http_server;
use settings::{SettingSpec, Settings};
use tls::ServerTlsConfig;

#[derive(Clone)]
pub(crate) struct RuntimeConfig {
    host: String,
    port: u16,
    data_dir: PathBuf,
    metrics_dir: Option<PathBuf>,
    flush_ms: u64,
    auth_enabled: bool,
    auth_secret_bytes: Option<Vec<u8>>,
    server_tls: Option<ServerTlsConfig>,
}

fn generate_self_signed_cert(out_dir: &Path) -> (PathBuf, PathBuf) {
    fs::create_dir_all(out_dir).unwrap();

    // Simple self-signed certificate for local usage
    let cert: CertifiedKey = generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let cert_path = out_dir.join("server.crt");
    let key_path = out_dir.join("server.key");

    fs::write(&cert_path, cert_pem).unwrap();
    fs::write(&key_path, key_pem).unwrap();

    (cert_path, key_path)
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let config_path = parse_config_arg();
    let config = load_runtime_config(config_path);
    run_server(config, None)
        .await
        .expect("server failed to start");
}

pub(crate) fn load_runtime_config(config_path: Option<PathBuf>) -> RuntimeConfig {
    let cwd = env::current_dir().expect("failed to get cwd");

    let default_data_dir = cwd.join("data_dir");

    let resolved_config = config_path.or_else(|| resolve_default_config(DEFAULT_SETTINGS_YAML));

    let settings = Settings::from_optional_file(resolved_config.as_ref()).unwrap_or_else(|e| {
        eprintln!("Failed to load settings: {e}");
        std::process::exit(1);
    });

    let host: String = settings
        .value(&SettingSpec {
            section: "server",
            key: "bind_host",
            env_var: "BIND_HOST",
            default: "0.0.0.0".to_string(),
        })
        .expect("failed to read host");

    let port: u16 = settings
        .value(&SettingSpec {
            section: "server",
            key: "bind_port",
            env_var: "BIND_PORT",
            default: 8245u16,
        })
        .expect("failed to read port");

    let data_dir: PathBuf = settings
        .value(&SettingSpec {
            section: "paths",
            key: "data_dir",
            env_var: "DATA_DIR",
            default: default_data_dir.clone(),
        })
        .expect("failed to read data_dir");

    let tls_enabled: bool = settings
        .value(&SettingSpec {
            section: "tls",
            key: "enabled",
            env_var: "TLS_ENABLE",
            default: false,
        })
        .expect("failed to read tls.enabled");

    let tls_auto: bool = settings
        .value(&SettingSpec {
            section: "tls",
            key: "auto",
            env_var: "TLS_AUTO",
            default: true,
        })
        .expect("failed to read tls.auto");

    let tls_cert_path: Option<PathBuf> = settings
        .value(&SettingSpec {
            section: "tls",
            key: "cert_path",
            env_var: "TLS_CERT_PATH",
            default: None::<PathBuf>,
        })
        .expect("failed to read tls.cert_path");

    let tls_key_path: Option<PathBuf> = settings
        .value(&SettingSpec {
            section: "tls",
            key: "key_path",
            env_var: "TLS_KEY_PATH",
            default: None::<PathBuf>,
        })
        .expect("failed to read tls.key_path");

    let metrics_dir: Option<PathBuf> = settings
        .value(&SettingSpec {
            section: "metrics",
            key: "dir",
            env_var: "METRICS_DIR",
            default: None::<PathBuf>,
        })
        .expect("failed to read metrics.dir");

    let flush_ms: u64 = settings
        .value(&SettingSpec {
            section: "persistence",
            key: "flush_ms",
            env_var: "PERSIST_FLUSH_MS",
            default: 15_000u64,
        })
        .expect("failed to read persistence.flush_ms");

    let auth_enabled: bool = settings
        .value(&SettingSpec {
            section: "auth",
            key: "enabled",
            env_var: "AUTH_ENABLED",
            default: false,
        })
        .expect("failed to read auth.enabled");

    let auth_secret_str: Option<String> = settings
        .value(&SettingSpec {
            section: "auth",
            key: "secret",
            env_var: "AUTH_SECRET",
            default: None::<String>,
        })
        .expect("failed to read auth.secret");

    let auth_secret_bytes = auth_secret_str.map(|s| s.into_bytes());

    let server_tls = if tls_enabled {
        let (cert_path, key_path) = match (tls_cert_path, tls_key_path, tls_auto) {
            (Some(cert_path), Some(key_path), _) => (cert_path, key_path),
            (None, None, true) => generate_self_signed_cert(&data_dir.join("certificates")),
            (None, None, false) => {
                eprintln!("TLS is enabled but no cert/key provided and TLS_AUTO=false; provide TLS_CERT_PATH and TLS_KEY_PATH or enable TLS_AUTO.");
                std::process::exit(1);
            }
            (Some(_), None, _) | (None, Some(_), _) => {
                eprintln!("Both TLS_CERT_PATH and TLS_KEY_PATH must be provided when TLS is enabled.");
                std::process::exit(1);
            }
        };
        Some(ServerTlsConfig::new(cert_path, key_path))
    } else {
        None
    };

    RuntimeConfig {
        host,
        port,
        data_dir,
        metrics_dir,
        flush_ms,
        auth_enabled,
        auth_secret_bytes,
        server_tls,
    }
}

async fn run_server(
    config: RuntimeConfig,
    ready_tx: Option<oneshot::Sender<Result<(), String>>>,
) -> Result<(), String> {
    let rt_db_dir = config.data_dir.join("rt_data");
    let static_db_file = config.data_dir.join("static.db");
    fs::create_dir_all(&rt_db_dir)
        .map_err(|e| format!("Failed to create data dir {}: {e}", rt_db_dir.display()))?;

    let http_tls = config.server_tls.clone();
    let http_schema = if http_tls.is_some() { "https" } else { "http" };
    let ws_schema = if http_tls.is_some() { "wss" } else { "ws" };
    info!("Starting unified server on {http_schema}/{ws_schema}://{}:{}", config.host, config.port);

    run_http_server(
        &config.host,
        config.port,
        &rt_db_dir,
        &static_db_file,
        http_tls,
        config.metrics_dir.clone(),
        config.flush_ms,
        config.auth_enabled,
        config.auth_secret_bytes.clone(),
        ready_tx,
    )
    .await
}

fn parse_config_arg() -> Option<PathBuf> {
    let mut args = env::args().skip(1);
    let mut config: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                if let Some(path) = args.next() {
                    config = Some(PathBuf::from(path));
                } else {
                    eprintln!("--config <path> requires a file path");
                    std::process::exit(2);
                }
            }
            _ if arg.starts_with("--config=") => {
                let (_, path) = arg.split_at("--config=".len());
                if path.is_empty() {
                    eprintln!("--config=<path> requires a file path");
                    std::process::exit(2);
                }
                config = Some(PathBuf::from(path));
            }
            _ if arg.starts_with("-c=") => {
                let (_, path) = arg.split_at("-c=".len());
                if path.is_empty() {
                    eprintln!("-c=<path> requires a file path");
                    std::process::exit(2);
                }
                config = Some(PathBuf::from(path));
            }
            unknown => {
                eprintln!("Unknown argument: {unknown}");
                std::process::exit(2);
            }
        }
    }
    config
}

fn resolve_default_config(_default_yaml: &str) -> Option<PathBuf> {
    let etc_cfg = PathBuf::from("/etc/lirays-scada/settings.yaml");
    if etc_cfg.exists() {
        Some(etc_cfg)
    } else {
        None
    }
}
