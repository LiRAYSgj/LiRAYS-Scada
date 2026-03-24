pub mod variable;
pub mod parser;
pub mod utils;
pub mod events;

use log::{error, info, warn};
use tokio::net::TcpStream;
use tokio::{net::TcpListener, select};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::{sync::Arc, collections::HashSet};
use prost::Message as ProstMessage;
use variable::{VariableManager};
use crate::rtdata::namespace::{Command, event::Ev};
use serde_json;

async fn handle_client_cmd(vm: Arc<VariableManager>, stream: TcpStream, addr: String) {
    match accept_async(stream).await {
        Ok(mut ws_stream) => {
            info!("Accepting client from {addr}");
            let mut events_rx = vm.events_tx.subscribe();
            let mut subscribed_set: HashSet<String> = HashSet::new();
            let mut get_tree_changes = false;
            let mut as_json = false;
            loop {
                select! {
                    cmd_result = ws_stream.next() => {
                        match cmd_result {
                            Some(Ok(msg)) => {
                                let (command, is_json_cmd) = match msg {
                                    Message::Binary(bin) => {
                                        match Command::decode(&*bin) {
                                            Ok(cmd) => (cmd, false),
                                            Err(_) => (Command { command_type: None }, false)
                                        }
                                    }
                                    Message::Text(txt) => {
                                        match serde_json::from_str(&txt.to_string()) {
                                            Ok(cmd) => (cmd, true),
                                            Err(_) => (Command { command_type: None }, true)
                                        }
                                    }
                                    _ => (Command { command_type: None }, false)
                                };
                                as_json = is_json_cmd;

                                let response = vm.exec_cmd(command, &mut subscribed_set, &mut get_tree_changes);
                                let resp_msg = if as_json {
                                    match serde_json::to_string(&response) {
                                        Ok(str_) => Message::Text(str_.into()),
                                        Err(e) => Message::Text(format!("Error: {e}").into())
                                    }
                                } else {
                                    Message::Binary(response.encode_to_vec().into())
                                };
                                match ws_stream.send(resp_msg).await {
                                    Ok(_) => (),
                                    Err(e) => error!("Error sending response. Err: {e}")
                                }
                            }
                            Some(Err(e)) => {
                                error!("Error reading next message from {addr}, Disconnecting. {e}");
                                break;
                            },
                            None => {
                                info!("Client from {addr} disconnected");
                                break;
                            }
                        }
                    }
                    data_result = events_rx.recv() => {
                        match data_result {
                            Ok(event) => {
                                let send_event = match &event.ev {
                                    Some(Ev::VarValueEv(var_id_val)) => subscribed_set.contains(&var_id_val.var_id),
                                    Some(Ev::TreeChangedEv(_)) => get_tree_changes,
                                    None => false
                                };
                                if send_event {
                                    let resp_msg = if as_json {
                                        match serde_json::to_string(&event) {
                                            Ok(str_) => Message::Text(str_.into()),
                                            Err(e) => Message::Text(format!("Error: {e}").into())
                                        }
                                    } else {
                                        Message::Binary(event.encode_to_vec().into())
                                    };
                                    match ws_stream.send(resp_msg).await {
                                        Ok(_) => (),
                                        Err(e) => error!("Error sending response client {addr}. Err: {e}")
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Error receiving message client {addr}: {e}")
                            }
                        }
                    }
                }

            }
        }
        Err(e) => error!("Error accepting client connection: {e}")
    }
}

pub async fn run_server(host: &str, port: u16, db_dir: &str) {
    let var_manager = Arc::new(VariableManager::new(db_dir));

    // println!("------------------");
    // for key in var_manager.list_keys_with_prefix("H:").unwrap() {
    //     println!("{}", key);
    // }
    // println!("------------------");
    // for key in var_manager.list_keys_with_prefix("D:").unwrap() {
    //     println!("{}", key);
    // }
    // println!("------------------");

    let listener_cmd = TcpListener::bind((host, port)).await.unwrap();

    loop {
        match listener_cmd.accept().await {
            Ok((stream, addr)) => {
                let vm = Arc::clone(&var_manager);
                tokio::spawn(async move {
                    handle_client_cmd(vm, stream, addr.to_string()).await
                });
            }
            Err(e) => {
                let msg = format!("Error accepting client connection: {e}");
                error!("{}", msg);
            }
        }
    };
}
