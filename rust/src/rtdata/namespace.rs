use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Meta {
    RootUid([u8; 16]),
    Vendor(String),
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub enum ItemType {
    #[default]
    Folder,
    Variable,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub enum VarDataType {
    #[default]
    Integer,
    Float,
    Text,
    Boolean
}

#[derive(Serialize, Deserialize)]
pub enum Value {
    Integer(i128),
    Float(f64),
    Text(String),
    Boolean(bool),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Item {
    #[serde(rename = "d")]
    pub id: String,
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,  // None only for root Item
    #[serde(rename = "c")]
    pub children: HashMap<String, (String, ItemType, Option<VarDataType>)>, // (child name: child id)
    #[serde(rename = "t")]
    pub i_type: ItemType,
    #[serde(rename = "v", skip_serializing_if = "Option::is_none")]
    pub var_d_type: Option<VarDataType>,
    #[serde(rename = "i", skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

#[derive(Deserialize)]
pub struct AddCommand {
    pub cmd_id: String,
    pub parent_id: String,
    pub items_meta: Vec<(String, ItemType, Option<VarDataType>)>,
}

#[derive(Serialize)]
pub struct AddResponse {
    pub cmd_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct ListCommand {
    pub cmd_id: String,
    pub item_id: Option<String>,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub cmd_id: String,
    pub children_folders: HashMap<String, String>,
    pub children_vars: HashMap<String, (String, VarDataType)>,
}

#[derive(Deserialize)]
pub struct SetCommand {
    pub cmd_id: String,
    pub var_ids_values: Vec<(String, Value)>,  // (id, value)
}

#[derive(Serialize)]
pub struct SetResponse {
    pub cmd_id: String,
}

#[derive(Deserialize)]
pub struct GetCommand {
    pub cmd_id: String,
    pub var_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct GetResponse {
    pub cmd_id: String,
    pub var_values: Vec<Option<Value>>,
}

#[derive(Deserialize)]
pub struct DelCommand {
    pub cmd_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct DelResponse {
    pub cmd_id: String,
}

#[derive(Deserialize)]
pub enum Command {
    ADD(AddCommand),
    LIST(ListCommand),
    SET(SetCommand),
    GET(GetCommand),
    DEL(DelCommand),
}

#[derive(Serialize)]
pub enum Response {
    ADD(AddResponse),
    LIST(ListResponse),
    SET(SetResponse),
    GET(GetResponse),
    DEL(DelResponse),
}
