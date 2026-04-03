use std::sync::Arc;

use axum::{
    extract::{State, ws::{Message as WsMessage, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use futures_util::StreamExt;
use log::{error, info, warn};
use prost::Message as ProstMessage;
use serde_json;

use crate::rtdata::{namespace::Command, should_send, variable::VariableManager};
use crate::http::AppState;

pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let vm = state.var_manager.clone();
    ws.on_upgrade(move |socket| async move {
        handle_ws_session(vm, socket).await;
    })
}

async fn handle_ws_session(vm: Arc<VariableManager>, mut socket: WebSocket) {
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
                                let response = vm.exec_cmd(command, &mut subscribed_set, &mut get_tree_changes).await;
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
                                let (command, is_json_cmd) = match serde_json::from_str::<Command>(&txt.to_string()) {
                                    Ok(cmd) => (cmd, true),
                                    Err(_) => (Command { command_type: None }, true),
                                };
                                as_json = is_json_cmd;
                                let response = vm.exec_cmd(command, &mut subscribed_set, &mut get_tree_changes).await;
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
                                    error!("Error sending Pong: {e}");
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
                        error!("WebSocket error: {e}");
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
                        let mut encoded: Vec<(&crate::rtdata::namespace::Event, Vec<u8>, String)> = Vec::with_capacity(batch.events.len());
                        for ev in batch.events.iter() {
                            let bin = ev.encode_to_vec();
                            let json = serde_json::to_string(ev).unwrap_or_else(|_| "{}".to_string());
                            encoded.push((ev, bin, json));
                        }

                        for (event, bin, json) in encoded.into_iter() {
                            if should_send(event, &subscribed_set, get_tree_changes) {
                                let resp_msg = if as_json {
                                    WsMessage::Text(json.clone().into())
                                } else {
                                    WsMessage::Binary(bin.clone().into())
                                };
                                if let Err(e) = socket.send(resp_msg).await {
                                    error!("Error sending response to client: {e}");
                                    break;
                                }
                            }
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
