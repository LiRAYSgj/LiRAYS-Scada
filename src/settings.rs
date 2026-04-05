use std::{env, fs, path::{Path, PathBuf}};

use log::warn;
use serde::de::DeserializeOwned;
use serde_yaml::Value;

/// Source where a value was ultimately obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    /// Value came from an environment variable.
    Env,
    /// Value came from the YAML configuration file.
    File,
    /// Value came from the provided default.
    Default,
}

/// Specification describing how to resolve a single setting.
///
/// - `section` / `key` locate the value inside the YAML file (if present).
/// - `env_var` names the environment variable that can override the YAML value.
/// - `default` is used only when neither env nor YAML provides a usable value.
#[derive(Debug, Clone)]
pub struct SettingSpec<'a, T> {
    pub section: &'a str,
    pub key: &'a str,
    pub env_var: &'a str,
    pub default: T,
}

/// Error type returned by the settings loader when the YAML file cannot be read or parsed.
#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    InvalidYaml(serde_yaml::Error),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsError::Io(err) => write!(f, "I/O error: {err}"),
            SettingsError::InvalidYaml(err) => write!(f, "Invalid YAML: {err}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<std::io::Error> for SettingsError {
    fn from(err: std::io::Error) -> Self {
        SettingsError::Io(err)
    }
}

impl From<serde_yaml::Error> for SettingsError {
    fn from(err: serde_yaml::Error) -> Self {
        SettingsError::InvalidYaml(err)
    }
}

/// Settings loader that layers: YAML file (if it exists) -> environment overrides -> defaults.
///
/// The loader keeps the parsed YAML in memory so multiple lookups only parse the file once.
#[derive(Debug, Clone)]
pub struct Settings {
    yaml_root: Option<Value>,
    source_path: Option<PathBuf>,
}

impl Settings {
    /// Build a loader from an optional file path. Missing files are treated as "no YAML provided".
    pub fn from_optional_file<P: AsRef<Path>>(path: Option<P>) -> Result<Self, SettingsError> {
        let Some(path_ref) = path.as_ref().map(|p| p.as_ref().to_path_buf()) else {
            return Ok(Self {
                yaml_root: None,
                source_path: None,
            });
        };

        if !path_ref.exists() {
            return Ok(Self {
                yaml_root: None,
                source_path: Some(path_ref),
            });
        }

        let content = fs::read_to_string(&path_ref)?;
        let yaml_root: Value = serde_yaml::from_str(&content)?;

        Ok(Self {
            yaml_root: Some(yaml_root),
            source_path: Some(path_ref),
        })
    }

    /// Resolve a value using the provided specification, returning the value and where it came from.
    pub fn resolve<'a, T>(&'a self, spec: &SettingSpec<'a, T>) -> Result<(T, ValueSource), SettingsError>
    where
        T: Clone + DeserializeOwned + EnvParse,
    {
        // 1) Environment variable overrides everything else
        if let Some(raw) = env::var_os(spec.env_var) {
            let raw_str = raw.to_string_lossy();
            match T::parse_env(&raw_str) {
                Ok(parsed) => return Ok((parsed, ValueSource::Env)),
                Err(err) => warn!(
                    "Env var {}='{}' could not be parsed: {}. Falling back to file/default.",
                    spec.env_var, raw_str, err
                ),
            }
        }

        // 2) YAML file value
        if let Some(val) = self.lookup_yaml_value(spec.section, spec.key) {
            match serde_yaml::from_value::<T>(val.clone()) {
                Ok(parsed) => return Ok((parsed, ValueSource::File)),
                Err(err) => warn!(
                    "YAML value at {}.{} could not be parsed: {}. Falling back to default.",
                    spec.section, spec.key, err
                ),
            }
        }

        // 3) Default
        Ok((spec.default.clone(), ValueSource::Default))
    }

    /// Convenience helper when you only need the value (and not the source).
    pub fn value<'a, T>(&'a self, spec: &SettingSpec<'a, T>) -> Result<T, SettingsError>
    where
        T: Clone + DeserializeOwned + EnvParse,
    {
        self.resolve(spec).map(|(v, _)| v)
    }

    fn lookup_yaml_value(&self, section: &str, key: &str) -> Option<&Value> {
        let Value::Mapping(root_map) = self.yaml_root.as_ref()? else {
            return None;
        };

        let section_val = root_map.get(&Value::String(section.to_string()))?;

        if let Value::Mapping(section_map) = section_val {
            section_map.get(&Value::String(key.to_string()))
        } else {
            None
        }
    }

    /// Expose the path that was attempted (if any) for logging or diagnostics.
    #[allow(dead_code)]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }
}

/// Trait describing how to parse a type from an environment variable string.
pub trait EnvParse: Sized {
    fn parse_env(raw: &str) -> Result<Self, String>;
}

macro_rules! impl_env_parse_from_str {
    ($($ty:ty),* $(,)?) => {
        $(impl EnvParse for $ty {
            fn parse_env(raw: &str) -> Result<Self, String> {
                raw.parse::<$ty>().map_err(|e| e.to_string())
            }
        })*
    };
}

impl_env_parse_from_str!(String, u8, u16, u32, u64, usize, i16, i32, i64, f32, f64);

impl EnvParse for PathBuf {
    fn parse_env(raw: &str) -> Result<Self, String> {
        Ok(PathBuf::from(raw))
    }
}

impl<T> EnvParse for Option<T>
where
    T: EnvParse,
{
    fn parse_env(raw: &str) -> Result<Self, String> {
        if raw.trim().is_empty() {
            Ok(None)
        } else {
            T::parse_env(raw).map(Some)
        }
    }
}

// Special-case bool to accept common truthy/falsey variants like "1"/"0".
impl EnvParse for bool {
    fn parse_env(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => Err(format!("invalid bool '{}', expected true/false/1/0", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let mut path = std::env::temp_dir();
        path.push(format!("{}_{}.yaml", name, ts));
        path
    }

    #[test]
    fn prefers_env_over_file_and_default() {
        let file_path = temp_path("settings_test");
        let mut file = File::create(&file_path).unwrap();
        write!(
            file,
            r#"
server:
  port: 9000
"#
        )
        .unwrap();

        let loader = Settings::from_optional_file(Some(&file_path)).unwrap();

        unsafe {
            std::env::set_var("TEST_PORT", "9100");
        }

        let spec = SettingSpec {
            section: "server",
            key: "port",
            env_var: "TEST_PORT",
            default: 8000u16,
        };

        let (val, source) = loader.resolve(&spec).unwrap();

        assert_eq!(val, 9100);
        assert_eq!(source, ValueSource::Env);

        unsafe {
            std::env::remove_var("TEST_PORT");
        }
        let (val_file, source_file) = loader.resolve(&spec).unwrap();
        assert_eq!(val_file, 9000);
        assert_eq!(source_file, ValueSource::File);

        let loader_empty = Settings::from_optional_file::<&Path>(None).unwrap();
        let (val_default, source_default) = loader_empty.resolve(&spec).unwrap();
        assert_eq!(val_default, 8000);
        assert_eq!(source_default, ValueSource::Default);
    }

    #[test]
    fn bool_env_accepts_numeric_variants() {
        let loader = Settings::from_optional_file::<&Path>(None).unwrap();
        unsafe {
            std::env::set_var("BOOL_TEST", "1");
        }

        let spec = SettingSpec {
            section: "feature",
            key: "enabled",
            env_var: "BOOL_TEST",
            default: false,
        };

        let (val, source) = loader.resolve(&spec).unwrap();
        assert!(val);
        assert_eq!(source, ValueSource::Env);
        unsafe {
            std::env::remove_var("BOOL_TEST");
        }
    }
}
