use std::{
    future::Future,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
};

use lirays_scada_proto::namespace::v1::value::Typed;
use serde::Deserialize;
use serde_json::json;
use tokio::net::{UdpSocket, lookup_host};

use super::{EgressBatch, EgressPlugin, Egressor};

/// Stays below a typical Ethernet MTU so datagrams are not fragmented.
const MAX_DATAGRAM_BYTES: usize = 1400;

pub struct UdpPlugin;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UdpConfig {
    host: String,
    port: u16,
}

struct UdpEgressor {
    config: UdpConfig,
    socket: Option<(UdpSocket, SocketAddr)>,
}

impl EgressPlugin for UdpPlugin {
    fn kind(&self) -> &'static str {
        "udp"
    }

    fn create(
        &self,
        _instance_id: &str,
        target: &serde_json::Value,
    ) -> Result<Box<dyn Egressor>, String> {
        let config: UdpConfig =
            serde_json::from_value(target.clone()).map_err(|e| e.to_string())?;
        if config.host.trim().is_empty() || config.port == 0 {
            return Err("host must not be blank and port must be positive".into());
        }
        Ok(Box::new(UdpEgressor {
            config,
            socket: None,
        }))
    }
}

impl UdpEgressor {
    async fn open_socket(&self) -> Result<(UdpSocket, SocketAddr), String> {
        let host = self.config.host.as_str();
        let target = lookup_host((host, self.config.port))
            .await
            .map_err(|e| format!("Cannot resolve {host}: {e}"))?
            .next()
            .ok_or_else(|| format!("No address found for {host}"))?;
        let local: SocketAddr = if target.is_ipv4() {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (Ipv6Addr::UNSPECIFIED, 0).into()
        };
        let socket = UdpSocket::bind(local).await.map_err(|e| e.to_string())?;
        Ok((socket, target))
    }
}

impl Egressor for UdpEgressor {
    fn publish<'a>(
        &'a mut self,
        batch: &'a EgressBatch,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            if self.socket.is_none() {
                self.socket = Some(self.open_socket().await?);
            }
            let (socket, target) = self.socket.as_ref().unwrap();
            for datagram in encode(batch) {
                if let Err(error) = socket.send_to(&datagram, target).await {
                    // Resolve the host again on the next attempt.
                    self.socket = None;
                    return Err(format!("UDP send failed: {error}"));
                }
            }
            Ok(())
        })
    }
}

/// Splits a batch into JSON datagrams: `{"ts":<ms>,"values":[{"path":..,"value":..}]}`.
/// A value too large for the limit is sent alone in its own datagram.
fn encode(batch: &EgressBatch) -> Vec<Vec<u8>> {
    let header = format!(r#"{{"ts":{},"values":["#, batch.timestamp_ms);
    let mut datagrams = Vec::new();
    let mut current = header.clone();
    for item in &batch.values {
        let value = match item.value.as_ref().and_then(|v| v.typed.as_ref()) {
            Some(Typed::IntegerValue(v)) => json!(v),
            Some(Typed::FloatValue(v)) => json!(v),
            Some(Typed::TextValue(v)) => json!(v),
            Some(Typed::BooleanValue(v)) => json!(v),
            None => serde_json::Value::Null,
        };
        let entry = json!({"path": item.var_id, "value": value}).to_string();
        let has_entries = current.len() > header.len();
        // One byte for the separator and two for the closing `]}`.
        if has_entries && current.len() + entry.len() + 3 > MAX_DATAGRAM_BYTES {
            current.push_str("]}");
            datagrams.push(std::mem::replace(&mut current, header.clone()).into_bytes());
        } else if has_entries {
            current.push(',');
        }
        current.push_str(&entry);
    }
    if current.len() > header.len() {
        current.push_str("]}");
        datagrams.push(current.into_bytes());
    }
    datagrams
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lirays_scada_proto::namespace::v1::{Value, VarIdValue};

    use super::*;

    #[tokio::test]
    async fn publishes_batches_as_bounded_json_datagrams() {
        let receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = receiver.local_addr().unwrap().port();
        let mut egressor = UdpPlugin
            .create("test", &json!({"host": "127.0.0.1", "port": port}))
            .unwrap();
        let values: Vec<_> = (0..100)
            .map(|i| VarIdValue {
                var_id: format!("/plant/area/line/variable_with_a_long_name_{i}"),
                value: Some(Value {
                    typed: Some(match i % 4 {
                        0 => Typed::IntegerValue(i),
                        1 => Typed::FloatValue(i as f64 + 0.5),
                        2 => Typed::TextValue(format!("text {i}")),
                        _ => Typed::BooleanValue(true),
                    }),
                }),
            })
            .collect();
        egressor
            .publish(&EgressBatch {
                timestamp_ms: 42,
                values,
            })
            .await
            .unwrap();

        let mut received = Vec::new();
        let mut buffer = [0u8; 65536];
        while received.len() < 100 {
            let len = tokio::time::timeout(Duration::from_secs(2), receiver.recv(&mut buffer))
                .await
                .unwrap()
                .unwrap();
            assert!(len <= MAX_DATAGRAM_BYTES);
            let datagram: serde_json::Value = serde_json::from_slice(&buffer[..len]).unwrap();
            assert_eq!(datagram["ts"], 42);
            received.extend(datagram["values"].as_array().unwrap().clone());
        }
        assert_eq!(received.len(), 100);
        assert_eq!(
            received[0],
            json!({"path": "/plant/area/line/variable_with_a_long_name_0", "value": 0})
        );
        assert_eq!(received[2]["value"], "text 2");
        assert_eq!(received[3]["value"], true);
    }

    #[test]
    fn invalid_targets_are_rejected() {
        for target in [
            json!({"host": "", "port": 9000}),
            json!({"host": "localhost", "port": 0}),
            json!({"host": "localhost"}),
            json!({"host": "localhost", "port": 9000, "extra": true}),
        ] {
            assert!(UdpPlugin.create("test", &target).is_err(), "{target}");
        }
    }
}
