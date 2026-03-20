use crate::rtdata::namespace::{EventType, SubscribeCommand};

use super::namespace::{ItemType, VarDataType, Command, ListCommand, command::CommandType};

pub fn cast_item_type(value: i32) -> ItemType {
    match ItemType::try_from(value) {
        Ok(it) => it,
        Err(_) => ItemType::Invalid
    }
}

pub fn cast_var_data_type(value: Option<i32>) -> VarDataType {
    match value {
        Some(v) => {
            match VarDataType::try_from(v) {
                Ok(dt) => dt,
                Err(_) => VarDataType::Invalid
            }
        }
        None => VarDataType::Invalid
    }
}

pub fn normalize_path(path: &str, i_type: ItemType) -> String {
    let trimmed = path.trim_matches('/');
    let components: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
    let mut base = format!("/{}", components.join("/"));
    match i_type {
        ItemType::Folder => {
            if !base.ends_with('/') {
                base.push('/')
            }
        },
        _ => ()
    }
    base
}

pub fn get_ancestors(path: &str) -> Vec<(String, String)> {
    let mut ancestors = vec![];
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    let mut parent = format!("/");
    for part in parts {
        ancestors.push((parent.clone(), part.to_string()));
        parent.push_str(format!("{}/", part).as_str());
    }
    ancestors
}

pub fn get_hierarchy_key(full_path: &str) -> String {
    let normalized = normalize_path(full_path, ItemType::Variable);
    let (parent, name) = normalized.rsplit_once('/').unwrap_or(("", &normalized));
    format!("H:{}/\0{}", parent, name)
}

pub fn generate_json_examples() {
    let cmd_examples = vec![
        Command { command_type: Some(CommandType::List(ListCommand {cmd_id: "cdwec".to_string(), folder_id: None}))},
        Command { command_type: Some(CommandType::Sub(SubscribeCommand {cmd_id: "cdwec".to_string(), events: vec![EventType::TreeChange as i32], var_ids: vec![], }))},
    ];
    for cmd in cmd_examples {
        let json = serde_json::to_string_pretty(&cmd).unwrap();
        println!("\n{}", json);
    }
}
