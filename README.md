# LiRAYS-Scada

## Running the Server Directly

You can run the Rust WebSocket server directly using the following command:

```sh
cargo run --bin server
```

The server accepts the following environment variables for configuration:

- `HOST` - Host address to bind to (default: "127.0.0.1")
- `PORT` - Port number to listen on (default: 9001)
- `DB_DIR` - Directory path for the database (default: "./db")

Example:

```sh
HOST=0.0.0.0 PORT=1236 DB_DIR=/var/lib/lirays cargo run --bin server
```

## Docker build

```
docker build --target production -t lirays:latest .
docker run --rm -p 8245:8245 -p 8246:8246 -v my_data_dir:/data --name lirays-scada lirays:latest
```

## Debian build

On a debian environment

```
make clean
make release
```

## General Schema

![General Schema](general_schema.png)

## Runtime Architecture

- Python entrypoint (`main.py`) starts:
  - Rust backend WebSocket server on `SCADA_RUST_HOST:SCADA_RUST_PORT` (default `0.0.0.0:1236`)
  - FastAPI HTTP API on `SCADA_API_HOST:SCADA_API_PORT` (default `0.0.0.0:1237`)
  - SvelteKit frontend (`frontend`) in SSR mode on `SCADA_FRONTEND_HOST:SCADA_FRONTEND_PORT` (default `0.0.0.0:3000`)
- The frontend communicates with Rust via:
  - `PUBLIC_DEMO_WS_ENDPOINT=ws://127.0.0.1:1236`
- Ports stay separated; a reverse proxy can later expose a single public port.

## Startup

### Node version (required)

Use Node `24` before running frontend commands:

```sh
nvm install 24
nvm use 24
```

### Frontend install/check/build

```sh
cd frontend
npm install
npm run check
npm run build
```

### Run all services through Python

```sh
python main.py
```

### Frontend mode toggles (optional)

`main.py` reads:

```env
SCADA_FRONTEND_MODE=production  # or development
SCADA_FRONTEND_PORT=3000
SCADA_FRONTEND_HOST=0.0.0.0
SCADA_FRONTEND_FORCE_BUILD=false
SCADA_FRONTEND_DIR=./frontend
SCADA_DEMO_DATA_DIR=./data_dir/rt_data

SCADA_RUST_HOST=0.0.0.0
SCADA_RUST_PORT=1236

SCADA_API_HOST=0.0.0.0
SCADA_API_PORT=1237
```

In `development` mode, Python runs:

```sh
npm run dev -- --host 0.0.0.0 --port 3000
```

In `production` mode, Python builds frontend if needed and runs:

```sh
npm run start
```

### Command examples

#### List Root

```json
{
  "command_type": {
    "List": {
      "cmd_id": "some-unique-id",
      "folder_id": null
    }
  }
}
```

#### List Folder

```json
{
  "command_type": {
    "List": {
      "cmd_id": "some-unique-id",
      "folder_id": "/Folder/Path"
    }
  }
}
```

Add Folders and Variables

```json
{
  "command_type": {
    "Add": {
      "cmd_id": "some-unique-id",
      "parent_id": "/Folder/Path",
      "items_meta": [
        ["fold1", 1, null],
        ["fold2", 1, null],
        ["int_var", 2, 1],
        ["float_var", 2, 2],
        ["text_var", 2, 3],
        ["bool_var", 2, 4]
      ]
    }
  }
}
```

Set Variable Values

```json
{
  "command_type": {
    "Set": {
      "cmd_id": "some-unique-id",
      "var_ids_values": [
        {
          "var_id": "/Folder/Path/int_var",
          "value": { "typed": { "IntegerValue": 23 } }
        },
        {
          "var_id": "/Folder/Path/float_var",
          "value": { "typed": { "FloatValue": -90.12 } }
        },
        {
          "var_id": "/Folder/Path/bool_var",
          "value": { "typed": { "BooleanValue": true } }
        },
        {
          "var_id": "/Folder/Path/text_var",
          "value": { "typed": { "TextValue": "Some text" } }
        }
      ]
    }
  }
}
```

Get Variable Values

```json
{
  "command_type": {
    "Get": {
      "cmd_id": "some-unique-id",
      "var_ids": [
        "/Folder/Path/int_var",
        "/Folder/Path/float_var",
        "/Folder/Path/bool_var",
        "/Folder/Path/text_var"
      ]
    }
  }
}
```

Delete Folders and Variables

```json
{
  "command_type": {
    "Del": {
      "cmd_id": "some-unique-id",
      "item_ids": [
        "/Folder/Path/fold1",
        "/Folder/Path/fold2",
        "/Folder/Path/int_var",
        "/Folder/Path/float_var",
        "/Folder/Path/bool_var",
        "/Folder/Path/text_var"
      ]
    }
  }
}
```

Subscribe to Variable Value changes

```json
{
  "command_type": {
    "Sub": {
      "cmd_id": "some-unique-id",
      "events": [1, 2],
      "var_ids": [
        "/Folder/Path/int_var",
        "/Folder/Path/float_var",
        "/Folder/Path/bool_var",
        "/Folder/Path/text_var"
      ]
    }
  }
}
```

Unsubscribe from Variable Value changes

```json
{
  "command_type": {
    "Unsub": {
      "cmd_id": "some-unique-id",
      "var_ids": [
        "/Folder/Path/int_var",
        "/Folder/Path/float_var",
        "/Folder/Path/bool_var",
        "/Folder/Path/text_var"
      ]
    }
  }
}
```

## New Rust http api examples

# Create a resource

curl -X POST http://localhost:8246/api/resources \
 -H "Content-Type: application/json" \
 -d '{"name":"Example Resource","description":"This is an example"}'

# Get all resources

curl http://localhost:8246/api/resources

# Get a specific resource

curl http://localhost:8246/api/resources/1

# Update a resource

curl -X PUT http://localhost:8246/api/resources/1 \
 -H "Content-Type: application/json" \
 -d '{"name":"Updated Resource","description":"Updated description"}'

# Delete a resource

curl -X DELETE http://localhost:8246/api/resources/1
