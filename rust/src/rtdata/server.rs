use log::{debug, error, info, warn};
use tokio::net::TcpStream;
use tokio::{runtime::Runtime, net::TcpListener, select};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::collections::HashSet;
use std::{sync::Arc, thread};
use prost::Message as ProstMessage;
use pyo3::prelude::*;
use super::variable::{VariableManager};
use super::namespace::{Command, command::CommandType};


async fn handle_client_cmd(vm: Arc<VariableManager>, stream: TcpStream, addr: String) {
    tokio::spawn(async move {
        match accept_async(stream).await {
            Ok(mut ws_stream) => {
                loop {
                    match ws_stream.next().await {
                        Some(Ok(msg)) => {
                            match msg {
                                Message::Binary(bin) => {
                                    let command = match Command::decode(&*bin) {
                                        Ok(cmd) => cmd,
                                        Err(_) => Command { command_type: None }
                                    };
                                    let response = vm.exec_cmd(command);
                                    let resp_bytes = response.encode_to_vec();
                                    match ws_stream.send(Message::Binary(resp_bytes.into())).await {
                                        Ok(_) => (),
                                        Err(e) => error!("Error sending response. Err: {e}")
                                    }
                                }
                                _ => ()
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
            }
            Err(e) => error!("Error accepting client connection: {e}")
        }
    });
}

async fn handle_client_sub(vm: Arc<VariableManager>, stream: TcpStream, addr: String) {
    tokio::spawn(async move {
        match accept_async(stream).await {
            Ok(mut ws_stream) => {
                let mut rx = vm.tx.subscribe();
                let mut subscribed_set: HashSet<String> = HashSet::new();
                loop {
                    select! {
                        cmd_result = ws_stream.next() => {
                            match cmd_result {
                                Some(Ok(msg)) => {
                                    match msg {
                                        Message::Binary(bin) => {
                                            let command = match Command::decode(&*bin) {
                                                Ok(cmd) => cmd,
                                                Err(_) => Command { command_type: None }
                                            };
                                            match command.command_type {
                                                Some(CommandType::Sub(sub_cmd)) => {
                                                    subscribed_set.extend(sub_cmd.var_ids);
                                                }
                                                Some(CommandType::Unsub(unsub_cmd)) => {
                                                    for _id in unsub_cmd.var_ids {
                                                        subscribed_set.remove(&_id);
                                                    }
                                                }
                                                _ => ()
                                            }
                                        }
                                        _ => ()
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
                        data_result = rx.recv() => {
                            match data_result {
                                Ok(msg) => {
                                    if subscribed_set.contains(&msg.var_id) {
                                        let resp_bytes = msg.encode_to_vec();
                                        match ws_stream.send(Message::Binary(resp_bytes.into())).await {
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
            Err(e) => error!("Error accepting client connection client {addr}: {e}")
        }
    });
}

async fn run_server(host: &str, port_cmd: u16, port_sub: u16, db_dir: &str) {
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

    let listener_cmd = TcpListener::bind((host, port_cmd)).await.unwrap();
    let listener_sub = TcpListener::bind((host, port_sub)).await.unwrap();

    info!("LiRAYS server for commands listening on {}:{}", host, port_cmd);
    info!("LiRAYS server for subscriptions listening on {}:{}", host, port_sub);

    loop {
        select! {
            result_a = listener_cmd.accept() => {
                match result_a {
                    Ok((stream, addr)) => {
                        debug!("Accepting cmd client from {addr}");
                        tokio::spawn(handle_client_cmd(var_manager.clone(), stream, addr.to_string()));
                    }
                    Err(e) => warn!("Error accepting cmd client: {e}")
                }
            }
            result_b = listener_sub.accept() => {
                match result_b {
                    Ok((stream, addr)) => {
                        debug!("Accepting sub client from {addr}");
                        tokio::spawn(handle_client_sub(var_manager.clone(), stream, addr.to_string()));
                    }
                    Err(e) => warn!("Error accepting sub client: {e}")
                }
            }
        }
    };
}

#[pyfunction]
pub fn serve(host: String, port_cmd: u16, port_sub: u16, db_file: String) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            run_server(&host, port_cmd, port_sub, &db_file).await;
        });
    });
}
