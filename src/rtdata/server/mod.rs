pub mod variable;
pub mod parser;
pub mod utils;
pub mod events;

use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use prost::Message as ProstMessage;
use serde_json;
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::{net::TcpListener, select};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tokio_rustls::{TlsAcceptor, rustls::{pki_types::{CertificateDer, PrivateKeyDer}, ServerConfig}};
use rustls_pemfile::{certs, private_key};
use variable::VariableManager;
use crate::rtdata::namespace::{event::Ev, Command};

#[derive(Clone)]
pub struct ServerTlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

impl ServerTlsConfig {
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self { cert_path, key_path }
    }
}

async fn handle_client_cmd<S>(vm: Arc<VariableManager>, stream: S, addr: String)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
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

pub fn build_tls_acceptor(config: &ServerTlsConfig) -> Result<TlsAcceptor, Box<dyn std::error::Error + Send + Sync>> {
    let mut cert_reader = std::io::BufReader::new(std::fs::File::open(&config.cert_path)?);
    let mut key_reader = std::io::BufReader::new(std::fs::File::open(&config.key_path)?);

    let certs: Vec<CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()?;

    let key: PrivateKeyDer<'static> = private_key(&mut key_reader)?
        .ok_or("no private key found in key file")?;

    let tls_config =ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(TlsAcceptor::from(Arc::new(tls_config)))
}

pub async fn run_server(host: &str, port: u16, db_dir: &str, tls_config: Option<ServerTlsConfig>) {
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

    let tls_acceptor = match tls_config {
        Some(cfg) => {
            match build_tls_acceptor(&cfg) {
                Ok(acceptor) => {
                    info!("TLS enabled for websocket server using cert {:?} and key {:?}", cfg.cert_path, cfg.key_path);
                    Some(acceptor)
                }
                Err(e) => {
                    panic!("Failed to set up TLS. Error: {e}");
                }
            }
        }
        None => None,
    };

    loop {
        match listener_cmd.accept().await {
            Ok((stream, addr)) => {
                let vm = Arc::clone(&var_manager);
                if let Some(acceptor) = tls_acceptor.clone() {
                    tokio::spawn(async move {
                        match acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                handle_client_cmd(vm, tls_stream, addr.to_string()).await
                            }
                            Err(e) => error!("TLS handshake failed for {addr}: {e}"),
                        }
                    });
                } else {
                    tokio::spawn(async move {
                        handle_client_cmd(vm, stream, addr.to_string()).await
                    });
                }
            }
            Err(e) => {
                let msg = format!("Error accepting client connection: {e}");
                error!("{}", msg);
            }
        }
    };
}
