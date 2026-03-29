pub mod namespace { include!(concat!(env!("OUT_DIR"), "/namespace.rs")); }
pub mod variable;
pub mod parser;
pub mod utils;
pub mod events;
pub mod metrics;

use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use prost::Message as ProstMessage;
use serde_json;
use std::{collections::HashSet, sync::Arc};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::{net::TcpListener, select};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message, Error as WsError, error::ProtocolError},
};
use variable::VariableManager;
use crate::rtdata::metrics::Metrics;
use super::rtdata::namespace::{event::Ev, Command, Event};
use super::tls::{ServerTlsConfig, build_tls_acceptor};

/// Decide if an event should be sent to a client based on current subscriptions.
fn should_send(event: &Event, subscribed_set: &HashSet<String>, get_tree_changes: bool) -> bool {
    match &event.ev {
        Some(Ev::VarValueEv(var_id_val)) => subscribed_set.contains(&var_id_val.var_id),
        Some(Ev::TreeChangedEv(_)) => get_tree_changes,
        _ => false,
    }
}

async fn handle_client_cmd<S>(vm: Arc<VariableManager>, stream: S, addr: String)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    match accept_async(stream).await {
        Ok(mut ws_stream) => {
            info!("Accepting client from {addr}");
            let mut events_rx = vm.register_listener().await;
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
                                let response = vm.exec_cmd(command, &mut subscribed_set, &mut get_tree_changes).await;
                                let resp_msg = if as_json {
                                    match serde_json::to_string(&response) {
                                        Ok(str_) => Message::Text(str_.into()),
                                        Err(e) => Message::Text(format!("Error: {e}").into())
                                    }
                                } else {
                                    Message::Binary(response.encode_to_vec().into())
                                };
                                // if ws_stream.
                                match ws_stream.send(resp_msg).await {
                                    Ok(_) => (),
                                    Err(_) => {
                                        info!("Client from {addr} disconnected");
                                        break;
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                match e {
                                    WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake) |
                                    WsError::ConnectionClosed => {
                                        info!("Client from {addr} disconnected");
                                    }
                                    other => {
                                        error!("Error reading next message from {addr}, Disconnecting. {other}");
                                    }
                                }
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
                            Some(batch) => {
                                // Pre-serialize once per batch per client.
                                let mut encoded: Vec<(&Event, Vec<u8>, String)> = Vec::with_capacity(batch.events.len());
                                for ev in batch.events.iter() {
                                    let bin = ev.encode_to_vec();
                                    let json = serde_json::to_string(ev).unwrap_or_else(|_| "{}".to_string());
                                    encoded.push((ev, bin, json));
                                }

                                for (event, bin, json) in encoded.into_iter() {
                                    if should_send(event, &subscribed_set, get_tree_changes) {
                                        let resp_msg = if as_json {
                                            Message::Text(json.clone().into())
                                        } else {
                                            Message::Binary(bin.clone().into())
                                        };
                                        if let Err(e) = ws_stream.send(resp_msg).await {
                                            error!("Error sending response client {addr}. Err: {e}");
                                            break;
                                        }
                                    }
                                }
                            }
                            None => {
                                warn!("Event channel closed for client {addr}");
                                break;
                            }
                        }
                    }
                }

            }
        }
        Err(e) => error!("Error accepting client connection: {e}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdata::namespace;
    use crate::rtdata::namespace::Event;

    #[test]
    fn should_send_respects_subscriptions() {
        let mut subs = HashSet::new();
        subs.insert("/a/b".to_string());
        let ev_var = Event { ev: Some(Ev::VarValueEv(namespace::VarIdValue { var_id: "/a/b".into(), value: None })) };
        let ev_other = Event { ev: Some(Ev::VarValueEv(namespace::VarIdValue { var_id: "/c".into(), value: None })) };
        let ev_tree = Event { ev: Some(Ev::TreeChangedEv(namespace::TreeChanged { folder_changed_event: vec![] })) };

        assert!(should_send(&ev_var, &subs, false));
        assert!(!should_send(&ev_other, &subs, false));
        assert!(!should_send(&ev_tree, &subs, false));
        assert!(should_send(&ev_tree, &subs, true));
    }
}

pub async fn run_server(host: &str, port: u16, db_dir: &str, tls_config: Option<ServerTlsConfig>) {
    let metrics = Arc::new(Metrics::new_from_env());
    if metrics.enabled() {
        Metrics::spawn_logger(metrics.clone());
    }
    let var_manager = Arc::new(VariableManager::new(db_dir, metrics));

    // Persist cache periodically
    let flush_ms: u64 = std::env::var("PERSIST_FLUSH_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(15_000);
    let _flush_handle = var_manager.clone().start_flush_loop(flush_ms);

    // Flush on shutdown signals (best effort)
    let vm_shutdown = var_manager.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
            let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
            tokio::select! {
                _ = sigterm.recv() => {},
                _ = sigint.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        info!("Shutdown signal received; flushing dirty cache");
        vm_shutdown.flush_dirty_now().await;
        info!("Cache flush complete; exiting");
        std::process::exit(0);
    });

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
