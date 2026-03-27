pub mod namespace { include!(concat!(env!("OUT_DIR"), "/namespace.rs")); }
pub mod types;

use futures_util::{SinkExt, StreamExt};
use prost::Message;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage, MaybeTlsStream, WebSocketStream};
use types::errors::ClientError;
use namespace::{
    command, response, value, Command, FolderInfo, ItemMeta, ItemType, OperationStatus, Response, VarDataType,
    VarIdValue, VarInfo,
};
use uuid::Uuid;

pub type WsSink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;
pub type WsStream = futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub struct Client {
    sink: Arc<Mutex<WsSink>>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Response>>>>,
    reader: JoinHandle<()>,
}

#[derive(Clone, Debug)]
pub struct IntegerVar {
    pub name: String,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct FloatVar {
    pub name: String,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct TextVar {
    pub name: String,
    pub unit: Option<String>,
    pub options: Vec<String>,
    pub max_len: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct BooleanVar {
    pub name: String,
    pub unit: Option<String>,
}

impl Client {
    pub async fn connect(host: &str, port: i64, tls: bool) -> Result<Self, ClientError> {
        let url = format!("{}://{}:{}", if tls {"wss"} else {"ws"}, host, port);
        let (ws, _) = connect_async(url).await?;
        let (sink, stream) = ws.split();

        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Response>>>> = Arc::new(Mutex::new(HashMap::new()));
        let pending_for_reader = Arc::clone(&pending);

        let reader = tokio::spawn(async move {
            let mut stream = stream;
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(WsMessage::Binary(data)) => {
                        if let Ok(resp) = Response::decode(&*data) {
                            if let Some(cmd_id) = extract_cmd_id_from_response(&resp) {
                                if let Some(tx) = pending_for_reader.lock().await.remove(cmd_id) {
                                    let _ = tx.send(resp);
                                }
                            }
                        }
                        // Ignore frames that are not valid responses for now (events or malformed).
                    }
                    Ok(WsMessage::Close(_)) => break,
                    Ok(WsMessage::Text(_)) => continue,
                    Err(_) => break,
                    _ => continue,
                }
            }
            // On exit we drop the pending senders; callers relying on timeouts will unblock eventually.
        });

        Ok(Self {
            sink: Arc::new(Mutex::new(sink)),
            pending,
            reader,
        })
    }

    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let mut sink: tokio::sync::MutexGuard<'_, WsSink> = self.sink.lock().await;
        sink.close().await?;
        self.reader.abort();
        Ok(())
    }

    /// Send a command and await its paired response (matched by `cmd_id`), then run `on_resp`.
    /// The command must carry a `cmd_id` inside its variant. Times out after `timeout_ms`.
    pub async fn _send_command<F, R>(
        &self,
        command: Command,
        timeout_ms: u64,
        on_resp: F,
    ) -> Result<R, ClientError>
    where
        F: FnOnce(Response) -> R + Send + 'static,
        R: Send + 'static,
    {
        let cmd_id = extract_cmd_id_from_command(&command).ok_or(ClientError::UnexpectedFrame)?.to_string();

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(cmd_id.clone(), tx);
        }

        let msg = WsMessage::Binary(command.encode_to_vec().into());
        {
            let mut sink = self.sink.lock().await;
            sink.send(msg).await?;
        }

        match timeout(Duration::from_millis(timeout_ms), rx).await {
            Ok(Ok(resp)) => Ok(on_resp(resp)),
            Ok(Err(_closed)) => Err(ClientError::ConnectionClosed),
            Err(_) => {
                // Timeout: clean up pending slot to avoid leaks.
                let mut pending = self.pending.lock().await;
                pending.remove(&cmd_id);
                Err(ClientError::Timeout)
            }
        }
    }

    /// List folders/variables under `folder_id` (root if None).
    /// Returns the raw protobuf structs from the server.
    pub async fn list(
        &self,
        folder_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<
        (
            Vec<FolderInfo>,
            Vec<VarInfo>,
        ),
        ClientError,
    > {
        let cmd_id = Uuid::new_v4().to_string();
        let cmd = Command {
            command_type: Some(command::CommandType::List(namespace::ListCommand {
                cmd_id: cmd_id.clone(),
                folder_id,
            })),
        };

        let res = self
            ._send_command(cmd, timeout_ms, |resp| {
                match resp.response_type {
                    Some(response::ResponseType::List(list_resp)) => {
                        Ok((list_resp.folders, list_resp.variables))
                    }
                    _ => Err(ClientError::UnexpectedFrame),
                }
            })
            .await?;

        res
    }

    /// Igual que `list`, pero devuelve datos primitivos para no exponer tipos protobuf.
    pub async fn list_raw(
        &self,
        folder_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<
        (
            Vec<(String, String)>, // (folder_id, name)
            Vec<(String, String, String, Option<String>, Option<f64>, Option<f64>, Vec<String>, Vec<u64>)>, // (var_id, name, var_d_type, unit, min, max, options, max_len)
        ),
        ClientError,
    > {
        let (folders, vars) = self.list(folder_id, timeout_ms).await?;

        let folders_mapped = folders.into_iter().map(|f| (f.id, f.name)).collect();

        let vars_mapped = vars
            .into_iter()
            .map(|v| {
                let v_type = VarDataType::try_from(v.var_d_type)
                    .map(|t| t.as_str_name().to_string())
                    .unwrap_or_else(|_| "VAR_DATA_TYPE_INVALID".to_string());
                (
                    v.id,
                    v.name,
                    v_type,
                    v.unit,
                    v.min,
                    v.max,
                    v.options,
                    v.max_len,
                )
            })
            .collect();

        Ok((folders_mapped, vars_mapped))
    }

    /// Crea una carpeta bajo `parent_id` (root si None) usando solo datos primitivos.
    /// Crea múltiples carpetas bajo `parent_id` en un solo AddCommand.
    pub async fn create_folders(
        &self,
        names: Vec<String>,
        parent_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let items: Vec<ItemMeta> = names
            .into_iter()
            .map(|name| ItemMeta {
                name,
                i_type: ItemType::Folder as i32,
                var_d_type: None,
                unit: None,
                min: None,
                max: None,
                options: vec![],
                max_len: vec![],
            })
            .collect();

        self.send_add(items, parent_id, timeout_ms).await
    }

    async fn send_add(
        &self,
        items_meta: Vec<ItemMeta>,
        parent_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let cmd_id = Uuid::new_v4().to_string();
        let cmd = Command {
            command_type: Some(command::CommandType::Add(namespace::AddCommand {
                cmd_id: cmd_id.clone(),
                parent_id,
                items_meta,
            })),
        };

        self._send_command(cmd, timeout_ms, |resp| {
            ensure_ok(&resp)?;
            match resp.response_type {
                Some(response::ResponseType::Add(_)) => Ok(()),
                _ => Err(ClientError::UnexpectedFrame),
            }
        })
        .await?
    }

    async fn send_set(
        &self,
        pairs: Vec<VarIdValue>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let cmd_id = Uuid::new_v4().to_string();
        let cmd = Command {
            command_type: Some(command::CommandType::Set(namespace::SetCommand {
                cmd_id: cmd_id.clone(),
                var_ids_values: pairs,
            })),
        };

        self._send_command(cmd, timeout_ms, |resp| {
            ensure_ok(&resp)?;
            match resp.response_type {
                Some(response::ResponseType::Set(_)) => Ok(()),
                _ => Err(ClientError::UnexpectedFrame),
            }
        })
        .await?
    }

    /// Crea variables enteras en lote.
    pub async fn create_integer_variables(
        &self,
        vars: Vec<IntegerVar>,
        parent_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let items = vars
            .into_iter()
            .map(|v| ItemMeta {
                name: v.name,
                i_type: ItemType::Variable as i32,
                var_d_type: Some(VarDataType::Integer as i32),
                unit: v.unit,
                min: v.min,
                max: v.max,
                options: vec![],
                max_len: vec![],
            })
            .collect();
        self.send_add(items, parent_id, timeout_ms).await
    }

    /// Crea variables float en lote.
    pub async fn create_float_variables(
        &self,
        vars: Vec<FloatVar>,
        parent_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let items = vars
            .into_iter()
            .map(|v| ItemMeta {
                name: v.name,
                i_type: ItemType::Variable as i32,
                var_d_type: Some(VarDataType::Float as i32),
                unit: v.unit,
                min: v.min,
                max: v.max,
                options: vec![],
                max_len: vec![],
            })
            .collect();
        self.send_add(items, parent_id, timeout_ms).await
    }

    /// Crea variables de texto en lote.
    pub async fn create_text_variables(
        &self,
        vars: Vec<TextVar>,
        parent_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let items = vars
            .into_iter()
            .map(|v| ItemMeta {
                name: v.name,
                i_type: ItemType::Variable as i32,
                var_d_type: Some(VarDataType::Text as i32),
                unit: v.unit,
                min: None,
                max: None,
                options: v.options,
                max_len: v.max_len,
            })
            .collect();
        self.send_add(items, parent_id, timeout_ms).await
    }

    /// Crea variables booleanas en lote.
    pub async fn create_boolean_variables(
        &self,
        vars: Vec<BooleanVar>,
        parent_id: Option<String>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let items = vars
            .into_iter()
            .map(|v| ItemMeta {
                name: v.name,
                i_type: ItemType::Variable as i32,
                var_d_type: Some(VarDataType::Boolean as i32),
                unit: v.unit,
                min: None,
                max: None,
                options: vec![],
                max_len: vec![],
            })
            .collect();
        self.send_add(items, parent_id, timeout_ms).await
    }

    /// Setea variables enteras en lote (misma cantidad de ids y valores).
    pub async fn set_integer_variables(
        &self,
        var_ids: Vec<String>,
        values: Vec<i64>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let pairs = build_pairs(var_ids, values, |v| value::Typed::IntegerValue(v))?;
        self.send_set(pairs, timeout_ms).await
    }

    /// Setea variables float en lote.
    pub async fn set_float_variables(
        &self,
        var_ids: Vec<String>,
        values: Vec<f64>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let pairs = build_pairs(var_ids, values, |v| value::Typed::FloatValue(v))?;
        self.send_set(pairs, timeout_ms).await
    }

    /// Setea variables de texto en lote.
    pub async fn set_text_variables(
        &self,
        var_ids: Vec<String>,
        values: Vec<String>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let pairs = build_pairs(var_ids, values, |v| value::Typed::TextValue(v))?;
        self.send_set(pairs, timeout_ms).await
    }

    /// Setea variables booleanas en lote.
    pub async fn set_boolean_variables(
        &self,
        var_ids: Vec<String>,
        values: Vec<bool>,
        timeout_ms: u64,
    ) -> Result<(), ClientError> {
        let pairs = build_pairs(var_ids, values, |v| value::Typed::BooleanValue(v))?;
        self.send_set(pairs, timeout_ms).await
    }
}

fn build_pairs<T, F>(
    var_ids: Vec<String>,
    values: Vec<T>,
    to_typed: F,
) -> Result<Vec<VarIdValue>, ClientError>
where
    F: Fn(T) -> value::Typed,
{
    if var_ids.len() != values.len() {
        return Err(ClientError::InvalidInput(
            "var_ids and values must have same length",
        ));
    }
    let iter = var_ids.into_iter().zip(values.into_iter());
    let pairs = iter
        .map(|(id, v)| VarIdValue {
            var_id: id,
            value: Some(namespace::Value {
                typed: Some(to_typed(v)),
            }),
        })
        .collect();
    Ok(pairs)
}

fn ensure_ok(resp: &Response) -> Result<(), ClientError> {
    match OperationStatus::try_from(resp.status).unwrap_or(OperationStatus::Invalid) {
        OperationStatus::Ok => Ok(()),
        OperationStatus::Err | OperationStatus::Invalid => Err(ClientError::OperationFailed(
            resp.error_msg.clone().unwrap_or_else(|| "unknown error".into()),
        )),
    }
}

fn extract_cmd_id_from_command(cmd: &Command) -> Option<&str> {
    use command::CommandType::*;
    match cmd.command_type.as_ref()? {
        Add(c) => Some(&c.cmd_id),
        List(c) => Some(&c.cmd_id),
        Set(c) => Some(&c.cmd_id),
        Get(c) => Some(&c.cmd_id),
        Del(c) => Some(&c.cmd_id),
        AddBulk(c) => Some(&c.cmd_id),
        Sub(c) => Some(&c.cmd_id),
        Unsub(c) => Some(&c.cmd_id),
        EditMeta(c) => Some(&c.cmd_id),
    }
}

fn extract_cmd_id_from_response(resp: &Response) -> Option<&str> {
    use response::ResponseType::*;
    match resp.response_type.as_ref()? {
        Add(r) => Some(&r.cmd_id),
        List(r) => Some(&r.cmd_id),
        Set(r) => Some(&r.cmd_id),
        Get(r) => Some(&r.cmd_id),
        Del(r) => Some(&r.cmd_id),
        Inv(r) => Some(&r.cmd_id),
        AddBulk(r) => Some(&r.cmd_id),
        Sub(r) => Some(&r.cmd_id),
        Unsub(r) => Some(&r.cmd_id),
        EditMeta(r) => Some(&r.cmd_id),
    }
}
