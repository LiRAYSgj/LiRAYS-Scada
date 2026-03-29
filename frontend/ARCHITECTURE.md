# LiRAYS-SCADA Frontend Architecture (brief)

This doc targets contributors who want to understand how the UI consumes the runtime namespace/events.

## High-level data flow
- The SPA connects via WebSocket to the backend (`/` WS port) and subscribes to variable values and tree changes.
- Initial tree is fetched with `ListCommand` from root. Subsequent structural updates arrive as `TreeChangedEv` batches.
- Variable updates arrive as `VarValueEv` events; the UI keeps an in-memory map keyed by `var_id`.
- Bulk creation: UI can POST/command `AddBulk` using JSON shaped like `frontend/__mocks__/ns.json` (supports range slices `[start:end:step]` and option lists `[a,b,c]`).

## Event handling
- Tree changes: the frontend patches its tree model using `removed_items` and newly added folders/variables supplied in each `FolderChanged` entry.
- Value changes: debounced rendering; the UI only paints subscribed vars.
- Dropped events: backend may drop batches for slow clients; UI should tolerate missed updates by occasional resync (re-list on detected gaps).

## Layout / components
- Tree panel: lazy loads children on expand via `ListCommand`.
- Detail/viewer: subscribes to selected vars; shows value/unit/constraints.
- Bulk import helper: can render a preview of JSON schema before sending `AddBulk`.

## Constraints / validation in UI
- Mirror backend constraints: min/max for numerics, options/max_len for text, boolean toggles.
- Display units when present; fall back to raw value.

## Dev tips
- Mock namespace: `frontend/__mocks__/ns.json` shows the accepted JSON shape with full variable metadata.
- Run backend in `cargo run --bin lirays-scada`; frontend dev server can proxy WS/HTTP to it.
- Metrics are backend-side; use terminal tail on `metrics_rt.txt` for quick performance checks.
