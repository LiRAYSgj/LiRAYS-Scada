use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Meta {
    RootUid([u8; 16]),
    Vendor(String),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Variable {
    #[serde(rename = "d")]
    pub id: String,
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(rename = "v")]
    pub children_vars: HashMap<String, String>, // (child name: child id)
    #[serde(rename = "i", skip_serializing_if = "Option::is_none")]
    pub int_v: Option<i128>,
    #[serde(rename = "f", skip_serializing_if = "Option::is_none")]
    pub float_v: Option<f64>,
    #[serde(rename = "b", skip_serializing_if = "Option::is_none")]
    pub bool_v: Option<bool>,
    #[serde(rename = "s", skip_serializing_if = "Option::is_none")]
    pub txt_v: Option<String>,
}

#[derive(Serialize)]
pub enum Value {
    Integer(i128),
    Float(f64),
    Text(String),
    Boolean(bool),
}

pub struct AddCommand {
    pub cmd_id: String,
    pub parent_id: String,
    pub var_names: Vec<String>,
}

pub struct ListCommand {
    pub cmd_id: String,
    pub var_id: Option<String>,
}

pub struct SetCommand {
    pub cmd_id: String,
    pub var_ids_values: Vec<(String, Value)>,  // (id, value)
}

pub struct GetCommand {
    pub cmd_id: String,
    pub var_ids: Vec<String>,
}

pub struct DelCommand {
    pub cmd_id: String,
    pub var_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct AddResponse {
    pub cmd_id: String,
    pub var_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub cmd_id: String,
    pub children: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct SetResponse {
    pub cmd_id: String,
}

#[derive(Serialize)]
pub struct GetResponse {
    pub cmd_id: String,
    pub var_values: Vec<Option<Value>>,
}

#[derive(Serialize)]
pub struct DelResponse {
    pub cmd_id: String,
}

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
