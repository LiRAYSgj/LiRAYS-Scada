use log::{debug, error, info};
use tokio::{runtime::Runtime, io::{AsyncReadExt, AsyncWriteExt}, sync::Mutex, net::TcpListener};
use std::{sync::Arc, thread};
use pyo3::prelude::*;
use crate::rtdata::{variable::VariableManager, parser::parse_command};


async fn run_server(host: &str, port: u16, db_dir: &str) {
    let var_manager = Arc::new(Mutex::new(VariableManager::new(db_dir)));
    match TcpListener::bind((host, port)).await {
        Ok(listener) => {
            info!("LiRAYS server listening on {}:{}", host, port);
            loop {
                match listener.accept().await {
                    Ok((mut socket, addr)) => {
                        debug!("Accepting client from {}", addr);
                        let var_manager = Arc::clone(&var_manager);
                        tokio::spawn(async move {
                            let mut buffer: [u8; 1024] = [0; 1024];
                            loop {
                                let n = match socket.read(&mut buffer).await {
                                    Ok(0) => {
                                        info!("Client from {} disconnected", addr);
                                        return;
                                    }
                                    Ok(n) => n,
                                    Err(e) => {
                                        info!("Client from {} disconnected with error reading: {}", addr, e);
                                        return;
                                    }
                                };

                                let response = match std::str::from_utf8(&buffer[..n]) {
                                    Ok(mut cmd) => {
                                        let vm = var_manager.lock().await;
                                        match parse_command(&mut cmd) {
                                            Ok(command) => {
                                                match vm.exec_cmd(command) {
                                                    Ok(resp) => {
                                                        match serde_json::to_string(&resp) {
                                                            Ok(s) => s,
                                                            Err(e) => format!("ER {e}")
                                                        }
                                                    },
                                                    Err(e) => format!("ER {e}")
                                                }
                                            }
                                            Err(e) => format!("ER {e}")
                                        }
                                    }
                                    Err(e) => format!("ER {e}")
                                };
                                match socket.write_all(response.as_bytes()).await {
                                    Ok(()) => (),
                                    Err(e) => {
                                        let msg = format!("Error sending response: {e}");
                                        error!("{}", msg);
                                    }
                                };
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
