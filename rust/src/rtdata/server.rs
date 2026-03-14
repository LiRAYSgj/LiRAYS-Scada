use log::{debug, error, info};
use tokio::{runtime::Runtime, sync::Mutex, net::TcpListener};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::{sync::Arc, thread};
use heed::{BytesDecode, BytesEncode};
use pyo3::prelude::*;
use super::variable::{VariableManager};
use super::proto::Proto;
use super::namespace::{Command, Response};


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
                                                    Message::Binary(bin) => {
                                                        let command = match Proto::<Command>::bytes_decode(&bin) {
                                                            Ok(cmd) => cmd,
                                                            Err(_) => Command { command_type: None }
                                                        };
                                                        let vm = var_manager.lock().await;
                                                        let response = vm.exec_cmd(command);
                                                        match Proto::<Response>::bytes_encode(&response) {
                                                            Ok(resp) => {
                                                                match ws_stream.send(Message::Binary(resp.into_owned().into())).await {
                                                                    Ok(_) => (),
                                                                    Err(e) => error!("Error sending response. Err: {e}")
                                                                }
                                                            },
                                                            Err(e) => error!("Error encoding response. Err: {e}")
                                                        };
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
