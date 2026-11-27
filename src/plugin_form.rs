//! Declarative configuration forms that plugins expose so clients can render
//! create and edit forms without plugin-specific UI code.
//!
//! A form only describes the configuration; the plugin's `create` remains the
//! authoritative validation.

use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Serialize, ToSchema)]
pub struct PluginDescriptor {
    pub plugin_type: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `None` when the plugin declares no form: clients edit the raw JSON configuration.
    #[schema(value_type = Option<Vec<Object>>)]
    pub fields: Option<Vec<FormField>>,
}

impl PluginDescriptor {
    /// A descriptor without a form.
    pub fn new(plugin_type: &str) -> Self {
        Self {
            plugin_type: plugin_type.into(),
            display_name: plugin_type.into(),
            description: None,
            fields: None,
        }
    }

    pub fn with_form(
        plugin_type: &str,
        display_name: &str,
        description: &str,
        fields: Vec<FormField>,
    ) -> Self {
        Self {
            plugin_type: plugin_type.into(),
            display_name: display_name.into(),
            description: Some(description.into()),
            fields: Some(fields),
        }
    }
}

/// One configuration key. `key` is the JSON object key inside the enclosing object.
#[derive(Clone, Serialize)]
pub struct FormField {
    pub key: &'static str,
    pub label: &'static str,
    #[serde(flatten)]
    pub kind: FieldKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    /// Hidden fields are omitted from the submitted configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_when: Option<VisibleWhen>,
    /// For `tag` and `tag_list`: a folder dropped from the namespace tree becomes the
    /// pattern `folder/**`, selecting every variable below it, including future ones.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub folder_as_pattern: bool,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Textarea,
    /// Never returned by the instance API; omitted on update to keep the stored value.
    /// Only supported at the top level or inside groups, not inside lists.
    Secret,
    Number {
        integer: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
    },
    Boolean,
    Select {
        options: Vec<SelectOption>,
    },
    /// A namespace path or pattern.
    Tag,
    /// An array of namespace paths or patterns.
    TagList,
    /// A free JSON object.
    Json,
    /// A nested object.
    Group {
        fields: Vec<FormField>,
    },
    /// An array of objects.
    List {
        fields: Vec<FormField>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_items: Option<u32>,
    },
}

#[derive(Clone, Serialize)]
pub struct SelectOption {
    pub value: &'static str,
    pub label: &'static str,
}

/// Shows a field only while a sibling field holds one of the listed values.
#[derive(Clone, Serialize)]
pub struct VisibleWhen {
    pub field: &'static str,
    pub one_of: Vec<Value>,
}

impl FormField {
    pub fn new(key: &'static str, label: &'static str, kind: FieldKind) -> Self {
        Self {
            key,
            label,
            kind,
            description: None,
            required: false,
            default: None,
            placeholder: None,
            visible_when: None,
            folder_as_pattern: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn default_value(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    pub fn description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn placeholder(mut self, placeholder: &'static str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    pub fn folder_as_pattern(mut self) -> Self {
        self.folder_as_pattern = true;
        self
    }

    pub fn visible_when(mut self, field: &'static str, one_of: Vec<Value>) -> Self {
        self.visible_when = Some(VisibleWhen { field, one_of });
        self
    }
}

impl FieldKind {
    pub fn integer(min: Option<f64>, max: Option<f64>) -> Self {
        Self::Number {
            integer: true,
            min,
            max,
            step: Some(1.0),
        }
    }

    pub fn float(min: Option<f64>, max: Option<f64>) -> Self {
        Self::Number {
            integer: false,
            min,
            max,
            step: None,
        }
    }

    pub fn select(options: &[(&'static str, &'static str)]) -> Self {
        Self::Select {
            options: options
                .iter()
                .map(|&(value, label)| SelectOption { value, label })
                .collect(),
        }
    }
}

/// Removes `secret` values from a configuration before it leaves the server.
pub fn redact_secrets(fields: &[FormField], config: &mut Value) {
    let Value::Object(object) = config else {
        return;
    };
    for field in fields {
        match &field.kind {
            FieldKind::Secret => {
                object.remove(field.key);
            }
            FieldKind::Group { fields } => {
                if let Some(nested) = object.get_mut(field.key) {
                    redact_secrets(fields, nested);
                }
            }
            _ => {}
        }
    }
}

/// The configuration a client gets by accepting every default; used by plugin
/// tests to keep forms and configuration structs in sync.
#[cfg(test)]
pub(crate) fn default_config(fields: &[FormField]) -> Value {
    let mut object = serde_json::Map::new();
    for field in fields {
        let value = match (&field.kind, &field.default) {
            (_, Some(value)) => value.clone(),
            (FieldKind::Group { fields }, None) => default_config(fields),
            _ => continue,
        };
        object.insert(field.key.into(), value);
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn secrets_are_redacted_at_top_level_and_in_groups() {
        let fields = vec![
            FormField::new("user", "User", FieldKind::Text),
            FormField::new("password", "Password", FieldKind::Secret),
            FormField::new(
                "auth",
                "Auth",
                FieldKind::Group {
                    fields: vec![FormField::new("token", "Token", FieldKind::Secret)],
                },
            ),
        ];
        let mut config = json!({"user": "a", "password": "p", "auth": {"token": "t", "x": 1}});
        redact_secrets(&fields, &mut config);
        assert_eq!(config, json!({"user": "a", "auth": {"x": 1}}));
    }

    #[test]
    fn fields_serialize_with_flattened_type() {
        let field = FormField::new("port", "Port", FieldKind::integer(Some(1.0), Some(65535.0)))
            .required()
            .visible_when("mode", vec![json!("polling")]);
        assert_eq!(
            serde_json::to_value(field).unwrap(),
            json!({
                "key": "port", "label": "Port", "type": "number", "integer": true,
                "min": 1.0, "max": 65535.0, "step": 1.0, "required": true,
                "visible_when": {"field": "mode", "one_of": ["polling"]}
            })
        );
    }
}
