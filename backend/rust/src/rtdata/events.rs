use std::collections::HashMap;

use super::namespace::{FolderInfo, ItemMeta, ItemType, VarInfo, Event, DelCommand, TreeChanged, FolderChanged, event::Ev};
use super::utils::{cast_var_data_type, normalize_path, get_parent_and_name};

pub fn extract_add_event(
    folder_id: &str,
    reload: bool,
    new_folders_imeta: Vec<ItemMeta>,
    new_variables_imeta: Vec<ItemMeta>
) -> Result<Event, String> {
    let new_folders = new_folders_imeta.iter().map(|i_meta| {
        FolderInfo {
            id: normalize_path(&format!("{}/{}", folder_id, i_meta.name), ItemType::Folder),
            name: i_meta.name.to_string()
        }
    }).collect();
    let new_variables = new_variables_imeta.iter().map(|i_meta| {
        let var_d_type = cast_var_data_type(i_meta.var_d_type);
        VarInfo {
            id: normalize_path(&format!("{}/{}", folder_id, i_meta.name), ItemType::Variable),
            name: i_meta.name.to_string(),
            var_d_type: var_d_type as i32
        }
    }).collect();

    let folders_changed = vec![FolderChanged {
        folder_id: folder_id.to_string(),
        reload,
        removed_items: vec![],
        new_folders,
        new_variables,
    }];

    Ok(Event {
        ev: Some(Ev::TreeChangedEv(TreeChanged {
            folder_changed_event: folders_changed
        }))
    })
}

pub fn extract_del_event(
    del_cmd: &DelCommand
) -> Result<Event, String> {
    let mut removed_data: HashMap<String, Vec<String>> = HashMap::new();
    for item_id in del_cmd.item_ids.clone() {
        let (parent, _) = get_parent_and_name(&item_id);
        match removed_data.get_mut(&parent) {
            Some(removed_items) => {
                removed_items.push(item_id);
            }
            None => {
                removed_data.insert(parent, vec![item_id]);
            }
        }
    }

    let event = Event {
        ev: Some(Ev::TreeChangedEv(TreeChanged {
            folder_changed_event: removed_data.iter().map(|(parent, removed)| {
                FolderChanged {
                    folder_id: parent.clone(),
                    reload: false,
                    removed_items: removed.clone(),
                    new_folders: vec![],
                    new_variables: vec![],
                }
            }).collect()
        }))
    };

    Ok(event)
}
