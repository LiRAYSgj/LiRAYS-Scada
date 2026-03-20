use log::warn;

use crate::rtdata::namespace::{FolderInfo, VarInfo, ItemType};

use super::namespace::{Event, AddCommand, TreeChanged, FolderChanged, event::Ev};
use super::utils::{cast_item_type, cast_var_data_type, normalize_path};

pub fn extract_add_event(add_cmd: AddCommand, prev_folder_count: usize) -> Result<Event, String> {
    let folder_id = add_cmd.parent_id.ok_or("Missing parent id".to_string())?;
    let mut new_folders = vec![];
    let mut new_variables = vec![];
    let new_count = add_cmd.items_meta.len();
    let reload = new_count > prev_folder_count;

    if !reload {
        for i_meta in add_cmd.items_meta {
            match cast_item_type(i_meta.i_type) {
                ItemType::Folder => {
                    let id = normalize_path(&format!("{}/{}", folder_id, i_meta.name), ItemType::Folder);
                    new_folders.push(FolderInfo { id, name: i_meta.name });
                }
                ItemType::Variable => {
                    let id = normalize_path(&format!("{}/{}", folder_id, i_meta.name), ItemType::Variable);
                    let var_d_type = cast_var_data_type(i_meta.var_d_type);
                    new_variables.push(VarInfo { id, name: i_meta.name, var_d_type: var_d_type as i32 });
                }
                ItemType::Invalid => warn!("Invalid item type")
            }
        }
    }

    let folders_changed = vec![FolderChanged {
        folder_id,
        reload,
        removed_folders: vec![],
        removed_variables: vec![],
        new_folders,
        new_variables,
    }];

    Ok(Event {
        ev: Some(Ev::TreeChangedEv(TreeChanged {
            folder_changed_event: folders_changed
        }))
    })
}
