use log::{debug, error, info};
use tokio::{runtime::Runtime, sync::Mutex, net::TcpListener};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::{sync::Arc, thread};
use pyo3::prelude::*;
use crate::rtdata::{namespace::Command, variable::VariableManager};


async fn run_server(host: &str, port: u16, db_dir: &str) {
    let var_manager = Arc::new(Mutex::new(VariableManager::new(db_dir)));
    match TcpListener::bind((host, port)).await {
        Ok(listener) => {
            info!("LiRAYS server listening on {}:{}", host, port);
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("Accepting client from {addr}");
                        let var_manager = Arc::clone(&var_manager);
                        tokio::spawn(async move {
                            match accept_async(stream).await {
                                Ok(mut ws_stream) => {
                                    loop {
                                        match ws_stream.next().await {
                                            Some(Ok(msg)) => {
                                                match msg {
                                                    Message::Text(text) => {
                                                        let resp = match serde_json::from_slice::<Command>(text.as_bytes()) {
                                                            Ok(command) => {
                                                                let vm = var_manager.lock().await;
                                                                match vm.exec_cmd(command) {
                                                                    Ok(resp) => {
                                                                        match serde_json::to_string(&resp) {
                                                                            Ok(s) => s,
                                                                            Err(e) => format!("{e}")
                                                                        }
                                                                    },
                                                                    Err(e) => format!("{e}")
                                                                }
                                                            },
                                                            Err(e) => format!("{e}")
                                                        };
                                                        ws_stream.send(resp.into()).await.unwrap();
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
                    },
                    Err(e) => {
                        let msg = format!("Error accepting client connection: {e}");
                        error!("{}", msg);
                    }
                };
            }
        },
        Err(e) => {
            let msg = format!("Error opening tcp listener on {host}:{port}: {e}");
            error!("{}", msg);
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
