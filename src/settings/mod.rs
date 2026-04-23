pub mod settings;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use log::LevelFilter;
use rcgen::{CertifiedKey, generate_simple_self_signed};
use serde::de::DeserializeOwned;
use settings::{SettingSpec, Settings};

use crate::{settings::settings::EnvParse, tls::ServerTlsConfig};

#[derive(Clone)]
pub struct RuntimeConfig {
    pub host: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub metrics_real_time: bool,
    pub metrics_historic: bool,
    pub flush_ms: u64,
    pub auth_enabled: bool,
    pub auth_access_ttl: u64,
    pub auth_refresh_ttl: u64,
    pub auth_secret_bytes: Option<Vec<u8>>,
    pub server_tls: Option<ServerTlsConfig>,
    pub log_level: LevelFilter,
}

fn generate_self_signed_cert(out_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    fs::create_dir_all(out_dir)
        .map_err(|e| format!("Failed creating TLS output dir {}: {e}", out_dir.display()))?;

    // Simple self-signed certificate for local usage
    let cert: CertifiedKey = generate_simple_self_signed(vec!["localhost".to_string()])
        .map_err(|e| format!("Failed generating self-signed TLS certificate: {e}"))?;
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let cert_path = out_dir.join("server.crt");
    let key_path = out_dir.join("server.key");

    fs::write(&cert_path, cert_pem)
        .map_err(|e| format!("Failed writing TLS cert {}: {e}", cert_path.display()))?;
    fs::write(&key_path, key_pem)
        .map_err(|e| format!("Failed writing TLS key {}: {e}", key_path.display()))?;

    Ok((cert_path, key_path))
}

fn read_setting<T>(settings: &Settings, spec: &SettingSpec<'_, T>, label: &str) -> T
where
    T: Clone + DeserializeOwned + EnvParse,
{
    settings.value(spec).unwrap_or_else(|e| {
        eprintln!("Failed to read {label}: {e}");
        std::process::exit(1);
    })
}

pub fn load_runtime_config(config_path: Option<PathBuf>) -> RuntimeConfig {
    let cwd = env::current_dir().unwrap_or_else(|e| {
        eprintln!("Failed to get current directory: {e}");
        std::process::exit(1);
    });

    let default_data_dir = cwd.join("data");

    let settings = Settings::from_optional_file(config_path.as_ref()).unwrap_or_else(|e| {
        eprintln!("Failed to load settings: {e}");
        std::process::exit(1);
    });

    let host: String = read_setting(
        &settings,
        &SettingSpec {
            section: "server",
            key: "bind_host",
            env_var: "BIND_HOST",
            default: "0.0.0.0".to_string(),
        },
        "host",
    );

    let port: u16 = read_setting(
        &settings,
        &SettingSpec {
            section: "server",
            key: "bind_port",
            env_var: "BIND_PORT",
            default: 8245u16,
        },
        "port",
    );

    let data_dir_opt: Option<PathBuf> = read_setting(
        &settings,
        &SettingSpec {
            section: "paths",
            key: "data_dir",
            env_var: "DATA_DIR",
            default: Some(default_data_dir.clone()),
        },
        "data_dir",
    );
    let data_dir = data_dir_opt.unwrap_or(default_data_dir.clone());

    let tls_enabled: bool = read_setting(
        &settings,
        &SettingSpec {
            section: "tls",
            key: "enabled",
            env_var: "TLS_ENABLE",
            default: false,
        },
        "tls.enabled",
    );

    let tls_auto: bool = read_setting(
        &settings,
        &SettingSpec {
            section: "tls",
            key: "auto",
            env_var: "TLS_AUTO",
            default: true,
        },
        "tls.auto",
    );

    let tls_cert_path: Option<PathBuf> = read_setting(
        &settings,
        &SettingSpec {
            section: "tls",
            key: "cert_path",
            env_var: "TLS_CERT_PATH",
            default: None::<PathBuf>,
        },
        "tls.cert_path",
    );

    let tls_key_path: Option<PathBuf> = read_setting(
        &settings,
        &SettingSpec {
            section: "tls",
            key: "key_path",
            env_var: "TLS_KEY_PATH",
            default: None::<PathBuf>,
        },
        "tls.key_path",
    );

    let metrics_real_time: bool = read_setting(
        &settings,
        &SettingSpec {
            section: "metrics",
            key: "real_time",
            env_var: "METRICS_REAL_TIME",
            default: false,
        },
        "metrics.real_time",
    );

    let metrics_historic: bool = read_setting(
        &settings,
        &SettingSpec {
            section: "metrics",
            key: "historic",
            env_var: "METRICS_HISTORIC",
            default: false,
        },
        "metrics.historic",
    );

    let flush_ms_raw: u64 = read_setting(
        &settings,
        &SettingSpec {
            section: "persistence",
            key: "flush_ms",
            env_var: "PERSIST_FLUSH_MS",
            default: 15_000,
        },
        "persistence.flush_ms",
    );
    let flush_ms = flush_ms_raw.max(5_000);

    let auth_enabled: bool = read_setting(
        &settings,
        &SettingSpec {
            section: "auth",
            key: "enabled",
            env_var: "AUTH_ENABLED",
            default: false,
        },
        "auth.enabled",
    );

    let auth_secret_str: Option<String> = read_setting(
        &settings,
        &SettingSpec {
            section: "auth",
            key: "secret",
            env_var: "AUTH_SECRET",
            default: None::<String>,
        },
        "auth.secret",
    );

    if auth_enabled && auth_secret_str.is_none() {
        eprintln!("AUTH_ENABLED=true requires auth.secret (or AUTH_SECRET env) to be set.");
        std::process::exit(1);
    }

    let auth_access_ttl: u64 = read_setting(
        &settings,
        &SettingSpec {
            section: "auth",
            key: "access_ttl",
            env_var: "ACCESS_TTL",
            default: 60 * 60,
        },
        "auth.access_ttl",
    );

    let auth_refresh_ttl: u64 = read_setting(
        &settings,
        &SettingSpec {
            section: "auth",
            key: "refresh_ttl",
            env_var: "REFRESH_TTL",
            default: 24 * 60 * 60,
        },
        "auth.refresh_ttl",
    );

    let log_level_raw: String = read_setting(
        &settings,
        &SettingSpec {
            section: "logger",
            key: "level",
            env_var: "LOGGER_LEVEL",
            default: "info".to_string(),
        },
        "logger.level",
    );
    let log_level = LevelFilter::parse_env(&log_level_raw).unwrap_or_else(|err| {
        eprintln!(
            "Invalid logger.level/LOGGER_LEVEL value '{}': {}. Expected one of: off, error, warn, info, debug, trace.",
            log_level_raw, err
        );
        std::process::exit(1);
    });

    let auth_secret_bytes = auth_secret_str.map(|s| s.into_bytes());

    let server_tls = if tls_enabled {
        let (cert_path, key_path) = match (tls_cert_path, tls_key_path, tls_auto) {
            (Some(cert_path), Some(key_path), _) => (cert_path, key_path),
            (None, None, true) => match generate_self_signed_cert(&data_dir.join("certificates")) {
                Ok(paths) => paths,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            },
            (None, None, false) => {
                eprintln!(
                    "TLS is enabled but no cert/key provided and TLS_AUTO=false; provide TLS_CERT_PATH and TLS_KEY_PATH or enable TLS_AUTO."
                );
                std::process::exit(1);
            }
            (Some(_), None, _) | (None, Some(_), _) => {
                eprintln!(
                    "Both TLS_CERT_PATH and TLS_KEY_PATH must be provided when TLS is enabled."
                );
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
        metrics_real_time,
        metrics_historic,
        flush_ms,
        auth_enabled,
        auth_access_ttl,
        auth_refresh_ttl,
        auth_secret_bytes,
        server_tls,
        log_level,
    }
}

pub fn parse_config_arg() -> Option<PathBuf> {
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
