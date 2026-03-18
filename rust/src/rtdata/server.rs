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
use super::namespace::Command;


async fn handle_client_cmd(vm: Arc<VariableManager>, stream: TcpStream, addr: String) {
    match accept_async(stream).await {
        Ok(mut ws_stream) => {
            debug!("Accepting client from {addr}");
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
                                        let response = vm.exec_cmd(command, &mut subscribed_set);
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
        Err(e) => error!("Error accepting client connection: {e}")
    }
}

async fn run_server(host: &str, port: u16, db_dir: &str) {
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

    info!("LiRAYS server for commands listening on {}:{}", host, port);

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

#[pyfunction]
pub fn serve(host: String, port: u16, db_file: String) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            run_server(&host, port, &db_file).await;
        });
    });
}
