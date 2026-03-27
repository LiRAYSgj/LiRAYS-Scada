# lirays-demo (Rust)

Small executable that exercises the `lirays-ws-client` library against a running LiRAYS-SCADA backend.

## How to run
```sh
cd clients/demo/rust
# Ensure the client library is built from the path dependency
cargo run --release
```

By default it connects to `ws://localhost:8245`. Override with:
```sh
LIRAYS_WS_URL=wss://your-host:8245 cargo run
```

What it does:
1. Connects via WebSocket.
2. Subscribes to `var1` for `VAR_VALUES` events (prints via callback).
3. Sends a `GetCommand` for `var1`.
4. Reads one incoming frame (response or event) and logs it.

## Referencing the library locally
The demo depends on the workspace library via a path dependency in `Cargo.toml`:
```toml
lirays-ws-client = { path = "../../rust-client" }
```
This uses the code in `/Users/alejandro/Local/LiRAYS/LiRAYS-Scada/clients/rust-client` without publishing to crates.io.

If you move the demo, adjust the relative path accordingly.
