use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde_yaml::{Mapping, Value};

use super::config::{load_settings, resolve_settings_write_path};
use super::prompt::print_table;
use crate::settings::settings::{EnvParse, SettingSpec, Settings, ValueSource};

const SECTION_ORDER: &[&str] = &["server", "paths", "tls", "metrics", "persistence", "auth"];

struct DisplayEntry {
    section: &'static str,
    parameter: &'static str,
    value: String,
    source: ValueSource,
    env_var: &'static str,
}

pub(super) async fn print_settings(config_arg: Option<PathBuf>) -> Result<(), String> {
    let (settings, used_path) = load_settings(config_arg)?;
    let source_label = used_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());

    println!("Settings source: {source_label}");

    let entries = collect_display_entries(&settings)?;
    for section in SECTION_ORDER {
        let section_rows: Vec<Vec<String>> = entries
            .iter()
            .filter(|entry| entry.section == *section)
            .map(|entry| {
                vec![
                    entry.parameter.to_string(),
                    entry.value.clone(),
                    format_source(entry.source).to_string(),
                    entry.env_var.to_string(),
                ]
            })
            .collect();
        if section_rows.is_empty() {
            continue;
        }
        println!();
        println!("[{section}]");
        print_table(&["parameter", "value", "source", "env_var"], &section_rows);
    }

    Ok(())
}

pub(super) async fn update_setting(
    config_arg: Option<PathBuf>,
    args: &[String],
) -> Result<(), String> {
    if args.len() < 2 {
        return Err("Usage: settings-update <section.key> <value>".to_string());
    }

    let key_path = &args[0];
    let raw_value = args[1..].join(" ");
    let (section, key) = parse_setting_path(key_path)?;
    let yaml_value = parse_yaml_value(&raw_value);

    let settings_path = resolve_settings_write_path(config_arg)?;
    update_yaml_setting(&settings_path, section, key, yaml_value.clone())?;

    println!("Updated settings file: {}", settings_path.display());
    println!("Set {section}.{key} = {}", format_yaml_value(&yaml_value));
    println!(
        "Run `lirays settings` to inspect values. Then run `lirays restart-service` to make them effective"
    );

    Ok(())
}

fn parse_setting_path(value: &str) -> Result<(&str, &str), String> {
    let Some((section, key)) = value.split_once('.') else {
        return Err(format!(
            "Invalid setting path '{value}'. Expected format: <section>.<parameter>"
        ));
    };
    if section.trim().is_empty() || key.trim().is_empty() {
        return Err(format!(
            "Invalid setting path '{value}'. Expected format: <section>.<parameter>"
        ));
    }
    Ok((section.trim(), key.trim()))
}

fn parse_yaml_value(raw: &str) -> Value {
    serde_yaml::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn format_yaml_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => serde_yaml::to_string(value)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "<complex>".to_string()),
    }
}

fn update_yaml_setting(path: &Path, section: &str, key: &str, value: Value) -> Result<(), String> {
    let mut root = if path.exists() {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read settings file {}: {e}", path.display()))?;
        if content.trim().is_empty() {
            Value::Mapping(Mapping::new())
        } else {
            serde_yaml::from_str::<Value>(&content)
                .map_err(|e| format!("Failed to parse settings file {}: {e}", path.display()))?
        }
    } else {
        Value::Mapping(Mapping::new())
    };

    let root_map = root.as_mapping_mut().ok_or_else(|| {
        format!(
            "Settings file {} has a non-mapping YAML root. Expected a map/object at top level.",
            path.display()
        )
    })?;

    let section_key = Value::String(section.to_string());
    if !root_map.contains_key(&section_key) {
        root_map.insert(section_key.clone(), Value::Mapping(Mapping::new()));
    }

    let section_value = root_map.get_mut(&section_key).ok_or_else(|| {
        format!(
            "Internal error while updating section '{section}' in {}",
            path.display()
        )
    })?;
    let section_map = section_value.as_mapping_mut().ok_or_else(|| {
        format!(
            "Section '{section}' in {} is not a map/object.",
            path.display()
        )
    })?;
    section_map.insert(Value::String(key.to_string()), value);

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
        }
    }

    let serialized = serde_yaml::to_string(&root)
        .map_err(|e| format!("Failed to serialize settings YAML: {e}"))?;
    fs::write(path, serialized)
        .map_err(|e| format!("Failed to write settings file {}: {e}", path.display()))
}

fn collect_display_entries(settings: &Settings) -> Result<Vec<DisplayEntry>, String> {
    let default_data_dir = env::current_dir()
        .map(|cwd| cwd.join("data"))
        .map_err(|e| format!("Failed to determine current directory: {e}"))?;

    let mut entries: Vec<DisplayEntry> = Vec::new();

    add_entry(
        &mut entries,
        settings,
        "server",
        "bind_host",
        "BIND_HOST",
        "0.0.0.0".to_string(),
        |v| v.clone(),
    )?;
    add_entry(
        &mut entries,
        settings,
        "server",
        "bind_port",
        "BIND_PORT",
        8245u16,
        |v| v.to_string(),
    )?;

    add_entry(
        &mut entries,
        settings,
        "paths",
        "data_dir",
        "DATA_DIR",
        Some(default_data_dir),
        |v: &Option<PathBuf>| match v {
            Some(path) => path.display().to_string(),
            None => "<none>".to_string(),
        },
    )?;

    add_entry(
        &mut entries,
        settings,
        "tls",
        "enabled",
        "TLS_ENABLE",
        false,
        |v| v.to_string(),
    )?;
    add_entry(
        &mut entries,
        settings,
        "tls",
        "auto",
        "TLS_AUTO",
        true,
        |v| v.to_string(),
    )?;
    add_entry(
        &mut entries,
        settings,
        "tls",
        "cert_path",
        "TLS_CERT_PATH",
        None::<PathBuf>,
        |v: &Option<PathBuf>| match v {
            Some(path) => path.display().to_string(),
            None => "<none>".to_string(),
        },
    )?;
    add_entry(
        &mut entries,
        settings,
        "tls",
        "key_path",
        "TLS_KEY_PATH",
        None::<PathBuf>,
        |v: &Option<PathBuf>| match v {
            Some(path) => path.display().to_string(),
            None => "<none>".to_string(),
        },
    )?;

    add_entry(
        &mut entries,
        settings,
        "metrics",
        "real_time",
        "METRICS_REAL_TIME",
        false,
        |v| v.to_string(),
    )?;

    add_entry(
        &mut entries,
        settings,
        "metrics",
        "historic",
        "METRICS_HISTORIC",
        false,
        |v| v.to_string(),
    )?;

    add_entry(
        &mut entries,
        settings,
        "persistence",
        "flush_ms",
        "PERSIST_FLUSH_MS",
        15_000u64,
        |v| v.to_string(),
    )?;

    add_entry(
        &mut entries,
        settings,
        "auth",
        "enabled",
        "AUTH_ENABLED",
        false,
        |v| v.to_string(),
    )?;
    add_entry(
        &mut entries,
        settings,
        "auth",
        "secret",
        "AUTH_SECRET",
        None::<String>,
        |v: &Option<String>| {
            if v.is_some() {
                "<set>".to_string()
            } else {
                "<unset>".to_string()
            }
        },
    )?;
    add_entry(
        &mut entries,
        settings,
        "auth",
        "access_ttl",
        "ACCESS_TTL",
        60 * 60,
        |v| v.to_string(),
    )?;
    add_entry(
        &mut entries,
        settings,
        "auth",
        "refresh_ttl",
        "REFRESH_TTL",
        24 * 60 * 60,
        |v| v.to_string(),
    )?;

    Ok(entries)
}

fn add_entry<T, F>(
    entries: &mut Vec<DisplayEntry>,
    settings: &Settings,
    section: &'static str,
    key: &'static str,
    env_var: &'static str,
    default: T,
    format_value: F,
) -> Result<(), String>
where
    T: Clone + DeserializeOwned + EnvParse,
    F: Fn(&T) -> String,
{
    let spec = SettingSpec {
        section,
        key,
        env_var,
        default,
    };
    let (value, source) = settings
        .resolve(&spec)
        .map_err(|e| format!("Failed to read {section}.{key}: {e}"))?;
    entries.push(DisplayEntry {
        section,
        parameter: key,
        value: format_value(&value),
        source,
        env_var,
    });
    Ok(())
}

fn format_source(source: ValueSource) -> &'static str {
    match source {
        ValueSource::Env => "env",
        ValueSource::File => "file",
        ValueSource::Default => "default",
    }
}
