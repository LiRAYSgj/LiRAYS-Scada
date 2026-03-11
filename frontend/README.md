# SCADA Frontend (SvelteKit SSR)

Interactive SCADA-style frontend built with SvelteKit + TypeScript.

This app is SSR-ready (`@sveltejs/adapter-node`) and is typically started by the root Python orchestrator (`main.py`) together with the Rust backend.

## Current Features

- Namespace browser (tree) backed by backend `LIST` commands over a single shared WebSocket
- Tree CRUD via WebSocket commands:
  - add node (`ADD`)
  - remove node (`DEL`)
- Graph workspace with drag/drop asset creation from tree nodes
- Edit/Play canvas modes:
  - **Edit**: build and modify graph
  - **Play**: realtime polling enabled
- Realtime value updates via `GET` polling every 2s (no subscription protocol)
- Write commands via `SET` from graph assets
- Typed graph input asset:
  - resolves input behavior from variable data type (`Text`, `Float`, `Integer`)
  - applies type/min/step rules
  - debounced writes (300ms), plus immediate commit on Enter/blur
- Light/Dark theming with CSS variable tokens

## Runtime Endpoints

Frontend expects backend WebSocket endpoint:

```env
PUBLIC_DEMO_WS_ENDPOINT=ws://127.0.0.1:1236
```

## Requirements

- Node `24+` (enforced by `package.json` engines)

Recommended with `nvm`:

```sh
nvm install 24
nvm use 24
```

## Development

Install deps:

```sh
npm install
```

Run dev server:

```sh
npm run dev
```

By default this serves on Vite defaults. If you run from root Python orchestration, the orchestrator handles frontend process startup and host/port settings.

## Production (SSR Node Server)

Build:

```sh
npm run build
```

Start SSR server:

```sh
npm run start
```

## Scripts

- `npm run dev` - start local dev server
- `npm run build` - production build
- `npm run start` - run SSR server from `build/`
- `npm run preview` - preview production build
- `npm run check` - Svelte + TypeScript checks
- `npm run lint` - Prettier + ESLint checks
- `npm run test` - unit tests (single run)
- `npm run test:unit` - Vitest watch/default mode
- `npm run demo:ws` - local mock WS backend for frontend-only testing

## Notes

- Frontend tree data is no longer mock/in-memory. It comes from backend `LIST`.
- Realtime flow no longer uses subscribe/unsubscribe commands. It uses polling (`GET`) every 2 seconds while Play mode is active.
- Canvas behavior includes lasso selection and middle-mouse pan; double-click zoom is disabled to avoid accidental zoom while interacting with numeric controls.

# SCADA UI Prototype

![Coverage](https://img.shields.io/badge/coverage-84.64%25-yellowgreen)

Interactive SCADA-style prototype built with SvelteKit + TypeScript.  
The app combines a lazy-loaded variable tree, a drag-and-drop process canvas, and realtime tag updates streamed over WebSocket.

## What This Project Includes

- Virtualized tree with async mock loading up to 10 levels
- Reusable IoC-driven context menus for tree and canvas drop actions
- Svelte Flow canvas with custom plant nodes (tank/pump/valve) and orthogonal pipe edges
- Edit/Play modes:
  - Edit: drag tags from tree to canvas and modify graph
  - Play: canvas becomes read-only and subscribes to live tag values
- Demo runtime uses:
  - WebSocket tag stream + writes
  - in-memory tree data adapter in frontend
- Light/Dark theming via design tokens

## Tech Stack

- SvelteKit + Svelte 5 runes
- TypeScript
- Tailwind CSS
- Vitest
- `ws` + `tsx` (demo realtime API server)
- `@xyflow/svelte` (canvas graph)
- `lucide-svelte` (icons)

## Getting Started

Use Node `24` with `nvm`:

```sh
nvm install 24
nvm use 24
```

```sh
npm install
```

Run frontend:

```sh
npm run dev
```

Run demo WebSocket API server:

```sh
npm run demo:ws
```

By default, the app expects:

```env
PUBLIC_DEMO_WS_ENDPOINT=ws://127.0.0.1:1236
DEMO_WS_SINE_ENABLED=true
```

## Scripts

- `npm run dev` - start local development server
- `npm run build` - production build
- `npm run start` - start SSR node server from `build/`
- `npm run preview` - preview production build
- `npm run lint` - prettier + eslint checks
- `npm run check` - svelte type checks
- `npm run test` - run unit tests once
- `npm run test -- --coverage` - run tests with coverage report
- `npm run demo:ws` - start mock websocket realtime server
