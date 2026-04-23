use std::sync::Arc;

use axum::{
    extract::{
        Extension, State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::StreamExt;
use lirays_scada_proto::namespace::v1::Command;
use log::{error, info, warn};
use prost::Message as ProstMessage;
use serde_json;

use crate::http::resources::user::model::Role;
use crate::http::{AppState, AuthContext};
use crate::rtdata::{should_send, variable::VariableManager};

fn is_expected_disconnect_error(message: &str) -> bool {
    let msg = message.to_ascii_lowercase();
    msg.contains("connection reset without closing handshake")
        || msg.contains("connection reset by peer")
        || msg.contains("broken pipe")
        || msg.contains("connection closed")
}

pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let vm = state.var_manager.clone();
    let role = auth.clone().map(|a| a.role.clone()).unwrap_or_else(|| {
        if state.auth.enabled {
            Role::Operator
        } else {
            Role::Admin
        }
    });
    ws.on_upgrade(move |socket| async move {
        handle_ws_session(vm, socket, role).await;
    })
}

async fn handle_ws_session(vm: Arc<VariableManager>, mut socket: WebSocket, role: Role) {
    let mut events_rx = vm.register_listener().await;
    let mut subscribed_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut get_tree_changes = false;
    let mut as_json = false;

    loop {
        tokio::select! {
            msg_result = socket.next() => {
                match msg_result {
                    Some(Ok(msg)) => {
                        match msg {
                            WsMessage::Binary(bin) => {
                                let (command, is_json_cmd) = match Command::decode(&*bin) {
                                    Ok(cmd) => (cmd, false),
                                    Err(_) => (Command { command_type: None }, false),
                                };
                                as_json = is_json_cmd;
                                let response = vm.exec_cmd(command, &mut subscribed_set, &mut get_tree_changes, &role).await;
                                let resp_msg = if as_json {
                                    match serde_json::to_string(&response) {
                                        Ok(str_) => WsMessage::Text(str_.into()),
                                        Err(e) => WsMessage::Text(format!("Error: {e}").into()),
                                    }
                                } else {
                                    WsMessage::Binary(response.encode_to_vec().into())
                                };
                                if let Err(e) = socket.send(resp_msg).await {
                                    error!("Error sending response to client: {e}");
                                    break;
                                }
                            }
                            WsMessage::Text(txt) => {
                                let (command, is_json_cmd) = match serde_json::from_str::<Command>(txt.as_ref()) {
                                    Ok(cmd) => (cmd, true),
                                    Err(_) => (Command { command_type: None }, true),
                                };
                                as_json = is_json_cmd;
                                let response = vm.exec_cmd(command, &mut subscribed_set, &mut get_tree_changes, &role).await;
                                let resp_msg = if as_json {
                                    match serde_json::to_string(&response) {
                                        Ok(str_) => WsMessage::Text(str_.into()),
                                        Err(e) => WsMessage::Text(format!("Error: {e}").into()),
                                    }
                                } else {
                                    WsMessage::Binary(response.encode_to_vec().into())
                                };
                                if let Err(e) = socket.send(resp_msg).await {
                                    error!("Error sending response to client: {e}");
                                    break;
                                }
                            }
                            WsMessage::Ping(payload) => {
                                if let Err(e) = socket.send(WsMessage::Pong(payload)).await {
                                    if is_expected_disconnect_error(&e.to_string()) {
                                        info!("Client disconnected during ping/pong: {e}");
                                    } else {
                                        error!("Error sending Pong: {e}");
                                    }
                                    break;
                                }
                            }
                            WsMessage::Close(_) => {
                                info!("Client closed connection");
                                break;
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        if is_expected_disconnect_error(&e.to_string()) {
                            info!("Client disconnected abruptly: {e}");
                        } else {
                            error!("WebSocket error: {e}");
                        }
                        break;
                    }
                    None => {
                        info!("Client disconnected");
                        break;
                    }
                }
            }
            data_result = events_rx.recv() => {
                match data_result {
                    Some(batch) => {
                        let mut disconnect = false;
                        for ev in batch.events.iter() {
                            if should_send(ev, &subscribed_set, get_tree_changes) {
                                let resp_msg = if as_json {
                                    WsMessage::Text(
                                        serde_json::to_string(ev)
                                            .unwrap_or_else(|_| "{}".to_string())
                                            .into(),
                                    )
                                } else {
                                    WsMessage::Binary(ev.encode_to_vec().into())
                                };
                                if let Err(e) = socket.send(resp_msg).await {
                                    error!("Error sending response to client: {e}");
                                    disconnect = true;
                                    break;
                                }
                            }
                        }
                        if disconnect {
                            break;
                        }
                    }
                    None => {
                        warn!("Event channel closed for client");
                        break;
                    }
                }
            }
        }
    }
}
