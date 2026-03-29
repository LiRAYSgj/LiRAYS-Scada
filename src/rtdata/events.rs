use std::collections::HashMap;

use crate::rtdata::namespace::{FolderInfo, ItemMeta, VarInfo, Event, DelCommand, TreeChanged, FolderChanged, event::Ev};
use super::utils::{normalize_path, get_parent_and_name};

/// Build a TreeChanged event for newly added folders/variables under `folder_id`.
/// Carries the metadata from ItemMeta (name/id/type; other meta left empty for now).
pub fn extract_add_event(
    folder_id: &str,
    reload: bool,
    new_folders_imeta: Vec<ItemMeta>,
    new_variables_imeta: Vec<ItemMeta>
) -> Result<Event, String> {
    let new_folders = new_folders_imeta.iter().map(|i_meta| {
        FolderInfo {
            id: normalize_path(&format!("{}/{}", folder_id, i_meta.name)),
            name: i_meta.name.to_string()
        }
    }).collect();
    let new_variables = new_variables_imeta.iter().map(|i_meta| {
        let var_d_type = i_meta.var_d_type();
        VarInfo {
            id: normalize_path(&format!("{}/{}", folder_id, i_meta.name)),
            name: i_meta.name.to_string(),
            var_d_type: var_d_type as i32,
            unit: i_meta.unit.clone(),
            min: i_meta.min,
            max: i_meta.max,
            options: i_meta.options.clone(),
            max_len: i_meta.max_len,
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

/// Build a TreeChanged event reflecting deletions grouped by parent folder.
/// Groups removed item ids by parent path to minimize event payload.
pub fn extract_del_event(
    del_cmd: &DelCommand
) -> Result<Event, String> {
    let mut removed_data: HashMap<String, Vec<String>> = HashMap::new();
    for item_id in del_cmd.item_ids.iter() {
        let (parent, _) = get_parent_and_name(item_id);
        match removed_data.get_mut(&parent) {
            Some(removed_items) => {
                removed_items.push(item_id.clone());
            }
            None => {
                removed_data.insert(parent, vec![item_id.clone()]);
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
