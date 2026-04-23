use std::{env, path::PathBuf};

use crate::settings::settings::{SettingSpec, Settings};

pub(super) fn load_settings(
    config_arg: Option<PathBuf>,
) -> Result<(Settings, Option<PathBuf>), String> {
    // Order of precedence:
    // 1) Explicit --settings / --config
    // 2) Platform default settings.yaml
    // 3) Platform default settings.default.yaml
    let primary = config_arg.or_else(discover_config_path);

    if let Some(path) = primary {
        if path.exists() {
            let settings = Settings::from_optional_file(Some(&path))
                .map_err(|e| format!("Failed to load settings {}: {e}", path.display()))?;
            return Ok((settings, Some(path)));
        }

        let mut fallback = path.clone();
        fallback.set_file_name("settings.default.yaml");
        if fallback.exists() {
            let settings = Settings::from_optional_file(Some(&fallback))
                .map_err(|e| format!("Failed to load settings {}: {e}", fallback.display()))?;
            return Ok((settings, Some(fallback)));
        }

        return Err(format!(
            "Settings file not found at {} (or {}). Use --settings <path>.",
            path.display(),
            fallback.display()
        ));
    }

    Err("No settings path could be determined; specify --settings <path>.".into())
}

pub(super) fn resolve_settings_write_path(config_arg: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = config_arg {
        return Ok(path);
    }

    discover_config_path().ok_or_else(|| {
        "No settings path could be determined; specify --settings <path>.".to_string()
    })
}

pub(super) fn resolve_data_dir(settings: &Settings) -> Result<PathBuf, String> {
    let default_data_dir = env::current_dir()
        .map(|cwd| cwd.join("data"))
        .map_err(|e| format!("Failed to determine current directory: {e}"))?;

    let dir_opt: Option<PathBuf> = settings
        .value(&SettingSpec {
            section: "paths",
            key: "data_dir",
            env_var: "DATA_DIR",
            default: Some(default_data_dir.clone()),
        })
        .map_err(|e| format!("Failed to read paths.data_dir: {e}"))?;

    Ok(dir_opt.unwrap_or(default_data_dir))
}

pub(super) fn build_setup_url(settings: &Settings) -> String {
    let host: String = settings
        .value(&SettingSpec {
            section: "server",
            key: "bind_host",
            env_var: "BIND_HOST",
            default: "127.0.0.1".to_string(),
        })
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let port: u16 = settings
        .value(&SettingSpec {
            section: "server",
            key: "bind_port",
            env_var: "BIND_PORT",
            default: 8245u16,
        })
        .unwrap_or(8245);

    let tls_enabled: bool = settings
        .value(&SettingSpec {
            section: "tls",
            key: "enabled",
            env_var: "TLS_ENABLE",
            default: false,
        })
        .unwrap_or(false);

    let display_host = if host == "0.0.0.0" {
        "127.0.0.1"
    } else {
        host.as_str()
    };
    let scheme = if tls_enabled { "https" } else { "http" };
    format!("{scheme}://{display_host}:{port}/auth/setup")
}

fn discover_config_path() -> Option<PathBuf> {
    if let Some(env_path) = env::var_os("LIRAYS_CONFIG") {
        return Some(PathBuf::from(env_path));
    }

    if cfg!(target_os = "macos") {
        Some(PathBuf::from(
            "/Library/Application Support/LiRAYSScada/settings.yaml",
        ))
    } else if cfg!(target_os = "linux") {
        Some(PathBuf::from("/etc/lirays-scada/settings.yaml"))
    } else {
        None
    }
}
