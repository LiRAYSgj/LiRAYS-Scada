# lirays-ws-client

Lightweight Rust client for connecting to the LiRAYS‑SCADA backend over WebSocket using Protobuf.

## Quick API

```rust
use lirays_ws_client::{Client, Incoming};
use lirays_ws_client::proto::namespace::{Command, command::CommandType, EventType, GetCommand};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::connect("ws://localhost:8245").await?;

    // Subscribe to variable changes and process events via callback
    client.subscribe(
        vec!["var1".into()],
        vec![EventType::EventTypeVarValues],
        Some(|ev| println!("event: {:?}", ev)),
    ).await?;

    // Send a manual command
    let cmd = Command { command_type: Some(CommandType::Get(GetCommand { cmd_id: "example".into(), var_ids: vec!["var1".into()] })) };
    client.write(cmd).await?;

    // Read a response/event (when no callback is active)
    if let Incoming::Response(resp) = client.read().await? {
        println!("response: {:?}", resp.status);
    }

    client.disconnect().await?;
    Ok(())
}
```

## Methods
- `connect(url)`: open WebSocket (`ws` or `wss`).
- `disconnect()`: close the session.
- `write(cmd)`: send a Protobuf `namespace::Command`.
- `read()`: wait for `Incoming::Response` or `Incoming::Event`.
- `subscribe(var_ids, events, callback)`: send `SubscribeCommand`; with a callback spawns a task that calls the callback per `Event` (same connection; avoid `read()` while callback is active).

### Dependencies
Tokio + tokio-tungstenite with TLS via rustls (native roots). Protobuf types are generated with `prost-build` from `../proto`.

### Development
```sh
cd clients/rust-client
cargo test
```
