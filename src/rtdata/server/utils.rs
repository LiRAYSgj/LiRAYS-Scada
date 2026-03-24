use crate::rtdata::namespace::{
    ItemType,
    VarDataType,
    Command,
    ListCommand,
    AddCommand,
    SetCommand,
    GetCommand,
    DelCommand,
    ListResponse,
    AddResponse,
    SetResponse,
    GetResponse,
    DelResponse,
    FolderInfo,
    VarInfo,
    ItemMeta,
    VarIdValue,
    OptionalValue,
    Value,
    command::CommandType,
    value::Typed,
};

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

fn _generate_json_examples(folder_path: &str) {
    use std::fs::File;
    use std::io::Write;

    // List Root
    let list_root_cmd = Command {
        command_type: Some(CommandType::List(ListCommand {
            cmd_id: "list-12345".to_string(),
            folder_id: None
        }))
    };
    if let Ok(json) = serde_json::to_string_pretty(&list_root_cmd) {
        let mut file = File::create(format!("{}/list_root.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }
    // List Folder
    let list_folder_cmd = Command {
        command_type: Some(CommandType::List(ListCommand {
            cmd_id: "list-12345".to_string(),
            folder_id: Some("/Folder/Path/".to_string())
        }))
    };
    if let Ok(json) = serde_json::to_string_pretty(&list_folder_cmd) {
        let mut file = File::create(format!("{}/list_folder.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // Add items
    let add_cmd = Command {
        command_type: Some(CommandType::Add(AddCommand {
            cmd_id: "add-123".to_string(),
            parent_id: Some("/devices/sensors/".to_string()),
            items_meta: vec![
                ItemMeta {
                    name: "NewFolder".to_string(),
                    i_type: ItemType::Folder as i32,
                    var_d_type: None
                },
                ItemMeta {
                    name: "NewFloatVariable".to_string(),
                    i_type: ItemType::Variable as i32,
                    var_d_type: Some(VarDataType::Float as i32)
                },
                ItemMeta {
                    name: "NewTextVariable".to_string(),
                    i_type: ItemType::Variable as i32,
                    var_d_type: Some(VarDataType::Text as i32)
                },
                ItemMeta {
                    name: "NewIntegerVariable".to_string(),
                    i_type: ItemType::Variable as i32,
                    var_d_type: Some(VarDataType::Integer as i32)
                },
                ItemMeta {
                    name: "NewBooleanVariable".to_string(),
                    i_type: ItemType::Variable as i32,
                    var_d_type: Some(VarDataType::Boolean as i32)
                }
            ],
        }))
    };
    if let Ok(json) = serde_json::to_string_pretty(&add_cmd) {
        let mut file = File::create(format!("{}/add.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // Set value
    let set_cmd = Command {
        command_type: Some(CommandType::Set(SetCommand {
            cmd_id: "set-456".to_string(),
            var_ids_values: vec![
                VarIdValue {
                    var_id: "/devices/sensors/NewFolder/NewFloatVariable".to_string(),
                    value: Some(Value {
                        typed: Some(Typed::FloatValue(23.12))
                    })
                },
                VarIdValue {
                    var_id: "/devices/sensors/NewFolder/NewTextVariable".to_string(),
                    value: Some(Value {
                        typed: Some(Typed::TextValue("some text".to_string()))
                    })
                },
                VarIdValue {
                    var_id: "/devices/sensors/NewFolder/NewIntegerVariable".to_string(),
                    value: Some(Value {
                        typed: Some(Typed::IntegerValue(23))
                    })
                },
                VarIdValue {
                    var_id: "/devices/sensors/NewFolder/NewBooleanVariable".to_string(),
                    value: Some(Value {
                        typed: Some(Typed::BooleanValue(true))
                    })
                }
            ],
        }))
    };
    if let Ok(json) = serde_json::to_string_pretty(&set_cmd) {
        let mut file = File::create(format!("{}/set.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // GetCommand example
    let get_cmd = Command {
        command_type: Some(CommandType::Get(GetCommand {
            cmd_id: "get-789".to_string(),
            var_ids: vec![
                "/devices/sensors/NewFolder/NewFloatVariable".to_string(),
                "/devices/sensors/NewFolder/NewTextVariable".to_string(),
                "/devices/sensors/NewFolder/NewIntegerVariable".to_string(),
                "/devices/sensors/NewFolder/NewBooleanVariable".to_string(),
            ],
        }))
    };
    if let Ok(json) = serde_json::to_string_pretty(&get_cmd) {
        let mut file = File::create(format!("{}/get.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // DelCommand example
    let del_cmd = Command {
        command_type: Some(CommandType::Del(DelCommand {
            cmd_id: "del-000".to_string(),
            item_ids: vec![
                "/devices/sensors/NewFolder/NewFloatVariable".to_string(),
                "/devices/sensors/NewFolder/NewTextVariable".to_string(),
                "/devices/sensors/NewFolder/NewIntegerVariable".to_string(),
                "/devices/sensors/NewFolder/NewBooleanVariable".to_string(),
            ],
        }))
    };
    if let Ok(json) = serde_json::to_string_pretty(&del_cmd) {
        let mut file = File::create(format!("{}/del.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // ListResponse example
    let list_resp = ListResponse {
        cmd_id: "list-12345".to_string(),
        folders: vec![
            FolderInfo {
                id: "/devices/sensors/NewFolder".to_string(),
                name: "NewFolder".to_string(),
            }
        ],
        variables: vec![
            VarInfo {
                id: "/devices/sensors/NewFolder/NewFloatVariable".to_string(),
                name: "NewFloatVariable".to_string(),
                var_d_type: VarDataType::Float as i32
            },
            VarInfo {
                id: "/devices/sensors/NewFolder/NewTextVariable".to_string(),
                name: "NewTextVariable".to_string(),
                var_d_type: VarDataType::Text as i32
            },
            VarInfo {
                id: "/devices/sensors/NewFolder/NewIntegerVariable".to_string(),
                name: "NewIntegerVariable".to_string(),
                var_d_type: VarDataType::Integer as i32
            },
            VarInfo {
                id: "/devices/sensors/NewFolder/NewBooleanVariable".to_string(),
                name: "NewBooleanVariable".to_string(),
                var_d_type: VarDataType::Boolean as i32
            }
        ],
    };
    if let Ok(json) = serde_json::to_string_pretty(&list_resp) {
        let mut file = File::create(format!("{}/list_response.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // AddResponse example
    let add_resp = AddResponse {
        cmd_id: "add-123".to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&add_resp) {
        let mut file = File::create(format!("{}/add_response.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // SetResponse example
    let set_resp = SetResponse {
        cmd_id: "set-456".to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&set_resp) {
        let mut file = File::create(format!("{}/set_response.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // GetResponse example
    let get_resp = GetResponse {
        cmd_id: "get-789".to_string(),
        var_values: vec![
            OptionalValue {
                value: Some(Value {
                    typed: Some(Typed::FloatValue(23.45))
                })
            },
            OptionalValue {
                value: Some(Value {
                    typed: Some(Typed::TextValue("some text".to_string()))
                })
            },
            OptionalValue {
                value: Some(Value {
                    typed: Some(Typed::IntegerValue(23))
                })
            },
            OptionalValue {
                value: Some(Value {
                    typed: Some(Typed::BooleanValue(true))
                })
            },
        ],
    };
    if let Ok(json) = serde_json::to_string_pretty(&get_resp) {
        let mut file = File::create(format!("{}/get_response.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    // DelResponse example
    let del_resp = DelResponse {
        cmd_id: "del-000".to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&del_resp) {
        let mut file = File::create(format!("{}/del_response.json", folder_path)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
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
