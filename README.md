# LiRAYS-Scada

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
List Root
```json
{
	"LIST": {
		"cmd_id": "some-unique-id"
	}
}
```
List Folder
```json
{
	"LIST": {
		"cmd_id": "some-unique-id",
		"item_id": "6caf355f-0d80-44dc-b58c-4598d6c32244"
	}
}
```
Add Folders and Variables
```json
{
	"ADD": {
		"cmd_id": "some-unique-id",
		"parent_id": "6caf355f-0d80-44dc-b58c-4598d6c32244",
		"items_meta": [
			["fold1", "Folder", null],
			["fold2", "Folder", null],
			["var1", "Variable", "Float"],
			["var2", "Variable", "Integer"],
			["var3", "Variable", "Text"],
			["var4", "Variable", "Boolean"]
		]
	}
}
```
Set Variable Values
```json
{
	"SET": {
		"cmd_id": "some-unique-id",
		"var_ids_values": [
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Integer": 23}],
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Float": -90.12}],
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Text": "Some text"}],
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Boolean": true}]
		]
	}
}
```
Get Variable Values
```json
{
	"GET": {
		"cmd_id": "some-unique-id",
		"var_ids": [
			"19d22f7f-6125-423c-8198-c7c5d41b8dea",
			"6ead3b82-e643-434d-ba9c-568187e8b895"
		]
	}
}
```
Delete Folders and Variables
```json
{
	"DEL": {
		"cmd_id": "some-unique-id",
		"item_ids": [
			"19d22f7f-6125-423c-8198-c7c5d41b8dea",
			"6ead3b82-e643-434d-ba9c-568187e8b895"
		]
	}
}
```

root:
    Area_[:20]:
        Line_[:4]:
            Motor_[:2]:
                Power: [Float]
                Speed: [Float]
                Direction: [Float]
                Temperature: [Float]
                Current: [Float]
                Voltage: [Float]
            Valve_[:4]:
                Position: [Float]
                Flow: [Float]
                Pressure: [Float]
                Temperature: [Float]
                Current: [Float]
                Voltage: [Float]
            Pump_[:2]:
                Power: [Float]
                Speed: [Float]
                Direction: [Float]
                Temperature: [Float]
                Current: [Float]
                Voltage: [Float]
            PGA:
                Var_[0:100]: [Float]