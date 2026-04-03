# lirays-ws-client

Lightweight Rust client for the LiRAYS-SCADA backend (WebSocket + Protobuf).

## Authentication-aware connect

```rust
use lirays_ws_client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) With auth enabled on the server (recommended)
    let client = Client::connect_with_credentials(
        "127.0.0.1",
        8245,
        false,          // tls?
        "admin",
        "secret-password",
    ).await?;

    // 2) Or if you already have a bearer token:
    // let client = Client::connect_with_token("127.0.0.1", 8245, false, Some(access_token)).await?;

    // 3) Or when auth is disabled on the server:
    // let client = Client::connect("127.0.0.1", 8245, false).await?;

    // do work ...
    client.disconnect().await?;
    Ok(())
}
```

The token is requested via `POST /auth/token` and then attached as `?token=...`
on the WebSocket URL so both HTTP → WS flows stay protected.

## Refresh flow

The server issues:
- access token: 1h TTL
- refresh token: 24h TTL

Use the helper to rotate automatically before expiry:

```rust
use lirays_ws_client::{refresh_tokens_with, TokenPair};

let TokenPair { access_token, refresh_token, .. } =
    refresh_tokens_with("127.0.0.1", 8245, false, refresh_token).await?;

// reconnect new WebSocket with the fresh access token
let client = Client::connect_with_token("127.0.0.1", 8245, false, Some(access_token)).await?;
```

## Quick capabilities

- Create/list/delete folders and variables in batch.
- Set/get variable values with typed helpers.
- Subscribe to value change events over a dedicated WebSocket.
- Bulk create from a JSON namespace schema.

See the demos in `demo/` for end-to-end usage with timeouts and subscriptions.

### Development

```sh
cd clients/rust-client
cargo check
```
