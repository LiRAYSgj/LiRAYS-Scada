use super::namespace::{FolderInfo, ItemMeta, ItemType, VarInfo};
use super::namespace::{Event, AddCommand, TreeChanged, FolderChanged, event::Ev};
use super::utils::{cast_var_data_type, normalize_path};

pub fn extract_add_event(
    add_cmd: AddCommand,
    reload: bool,
    new_folders_imeta: Vec<ItemMeta>,
    new_variables_imeta: Vec<ItemMeta>
) -> Result<Event, String> {
    let folder_id = add_cmd.parent_id.ok_or("Missing parent id".to_string())?;
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
