use crate::rtdata::namespace::{EventType, SubscribeCommand};

use super::namespace::{ItemType, VarDataType, Command, ListCommand, command::CommandType};

pub fn cast_item_type(value: i32) -> ItemType {
    ItemType::try_from(value).unwrap_or(ItemType::Invalid)
}

pub fn cast_var_data_type(value: Option<i32>) -> VarDataType {
    value.and_then(|v| VarDataType::try_from(v).ok()).unwrap_or(VarDataType::Invalid)
}

pub fn normalize_path(path: &str, i_type: ItemType) -> String {
    let mut base = String::with_capacity(path.len() + 2);
    base.push('/');
    
    let mut first = true;
    for part in path.split('/').filter(|s| !s.is_empty()) {
        if !first {
            base.push('/');
        }
        base.push_str(part);
        first = false;
    }
    
    if i_type == ItemType::Folder && !base.ends_with('/') {
        base.push('/');
    }
    base
}

pub fn get_ancestors(path: &str) -> Vec<(String, String)> {
    let mut ancestors = vec![];
    let mut parent = String::from("/");
    
    for part in path.split('/').filter(|s| !s.is_empty()) {
        ancestors.push((parent.clone(), part.to_string()));
        parent.push_str(part);
        parent.push('/');
    }
    ancestors
}

pub fn get_parent_and_name(path: &str) -> (String, String) {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match parts.as_slice() {
        [] => (String::from("/"), String::new()),
        [name] => (String::from("/"), name.to_string()),
        [parent_parts @ .., name] => {
            let mut parent = String::with_capacity(path.len());
            for part in parent_parts {
                parent.push('/');
                parent.push_str(part);
            }
            (parent, name.to_string())
        }
    }
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
        if let Ok(json) = serde_json::to_string_pretty(&cmd) {
            println!("\n{}", json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_item_type() {
        assert_eq!(cast_item_type(0), ItemType::Invalid);
        // Test with valid ItemType values if needed
    }

    #[test]
    fn test_cast_var_data_type() {
        assert_eq!(cast_var_data_type(None), VarDataType::Invalid);
        // Test with valid VarDataType values if needed
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/a/b/c", ItemType::Variable), "/a/b/c");
        assert_eq!(normalize_path("/a/b/c", ItemType::Folder), "/a/b/c/");
        assert_eq!(normalize_path("a/b/c", ItemType::Variable), "/a/b/c");
        assert_eq!(normalize_path("", ItemType::Variable), "/");
    }

    #[test]
    fn test_get_ancestors() {
        let ancestors = get_ancestors("/a/b/c");
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], (String::from("/"), String::from("a")));
        assert_eq!(ancestors[1], (String::from("/a/"), String::from("b")));
        assert_eq!(ancestors[2], (String::from("/a/b/"), String::from("c")));
    }

    #[test]
    fn test_get_parent_and_name() {
        // Test root path
        assert_eq!(get_parent_and_name("/"), (String::from("/"), String::from("")));
        
        // Test single component
        assert_eq!(get_parent_and_name("/a"), (String::from("/"), String::from("a")));
        
        // Test multiple components
        assert_eq!(get_parent_and_name("/a/b/c"), (String::from("/a/b"), String::from("c")));
        
        // Test path with trailing slash
        assert_eq!(get_parent_and_name("/a/b/c/"), (String::from("/a/b"), String::from("c")));
    }

    #[test]
    fn test_get_hierarchy_key() {
        assert_eq!(get_hierarchy_key("/a/b/c"), "H:/a/b/\0c");
        assert_eq!(get_hierarchy_key("/"), "H:/\0");
    }
}