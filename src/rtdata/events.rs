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
/// Groups removed item ids by normalized parent path to minimize payload.
pub fn extract_del_event(
    del_cmd: &DelCommand
) -> Result<Event, String> {
    let mut removed_data: HashMap<String, Vec<String>> = HashMap::new();
    for item_id in del_cmd.item_ids.iter() {
        let norm = normalize_path(item_id);
        let (parent, _) = get_parent_and_name(&norm);
        match removed_data.get_mut(&parent) {
            Some(removed_items) => {
                removed_items.push(norm.clone());
            }
            None => {
                removed_data.insert(parent, vec![norm]);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdata::namespace::DelCommand;

    #[test]
    fn add_event_builds_ids() {
        let folders = vec![ItemMeta { name: "f1".into(), i_type: 1, var_d_type: None, unit: None, min: None, max: None, options: vec![], max_len: None }];
        let vars = vec![ItemMeta { name: "v1".into(), i_type: 2, var_d_type: Some(1), unit: None, min: None, max: None, options: vec![], max_len: None }];
        let ev = extract_add_event("/Root", false, folders, vars).unwrap();
        match ev.ev.unwrap() {
            Ev::TreeChangedEv(tc) => {
                let fc = &tc.folder_changed_event[0];
                assert_eq!(fc.folder_id, "/Root");
                assert_eq!(fc.new_folders[0].id, "/Root/f1");
                assert_eq!(fc.new_variables[0].id, "/Root/v1");
            }
            _ => panic!("wrong ev"),
        }
    }

    #[test]
    fn del_event_groups_and_normalizes() {
        let dc = DelCommand { cmd_id: "c1".into(), item_ids: vec!["/Root/f1".into(), "/Root/f1/v1".into()] };
        let ev = extract_del_event(&dc).unwrap();
        match ev.ev.unwrap() {
            Ev::TreeChangedEv(tc) => {
                assert_eq!(tc.folder_changed_event.len(), 2);
                // Root folder change should include the direct child removal
                let root_change = tc.folder_changed_event.iter().find(|fc| fc.folder_id == "/Root").unwrap();
                assert!(root_change.removed_items.contains(&"/Root/f1".to_string()));
                // Nested folder change should include the variable
                let nested = tc.folder_changed_event.iter().find(|fc| fc.folder_id == "/Root/f1").unwrap();
                assert!(nested.removed_items.contains(&"/Root/f1/v1".to_string()));
            }
            _ => panic!("wrong ev"),
        }
    }
}
