# Frontend Architecture

This document describes the **current** architecture implemented in the Svelte frontend.

## Overview

- Framework: SvelteKit + Svelte 5 runes + TypeScript
- Rendering: SSR-capable app (`@sveltejs/adapter-node`) with client hydration for interactive SCADA screens
- Main route: `src/routes/+page.svelte`
- Styling: Tailwind CSS + route-level CSS variables (`src/routes/layout.css`)
- Graph engine: `@xyflow/svelte`
- Transport: single shared WebSocket client with command/response envelopes

## High-Level Runtime Model

The page composes three coordinated subsystems:

1. **Tree subsystem**
   - Browses namespace nodes from backend with `LIST`
   - Supports contextual actions (`Add`, `Remove`) through `ADD`/`DEL`
2. **Graph subsystem**
   - Accepts dropped tree nodes and instantiates plant assets
   - Shows live values and supports write actions (`SET`) on interactive assets
3. **Realtime subsystem**
   - Tracks graph tag IDs in Play mode
   - Polls backend every 2 seconds with `GET`
   - Publishes normalized values to Svelte stores

## Source Layout

```text
src/
  routes/
    +layout.svelte
    layout.css
    +page.svelte
  lib/
    core/
      theme/theme-utils.ts
      ws/
        types.ts
        command-ws-client.ts
        tag-stream-client.ts
    features/
      realtime/page-tag-realtime-provider.ts
      tree/
        types.ts
        flatten.ts
        tree-store.ts
        server-adapter.ts
        context-menu.ts
        components/
          VariableTree.svelte
          ContextMenu.svelte
          TreeRow.svelte
          TreeChevron.svelte
          TreeIcon.svelte
      graph/
        live-utils.ts
        components/PlantAssetNode.svelte
        assets/
          registry.ts
          types.ts
          controller.ts
          components/*.svelte
      workspace/components/PageToolbar.svelte
```

## WebSocket Architecture

### Protocol

The frontend uses command envelopes defined in `src/lib/core/ws/types.ts`:

- `LIST` for tree children fetch
- `GET` for polling live values
- `SET` for writing values
- `ADD` for creating nodes
- `DEL` for deleting nodes

### Shared Client

`src/lib/core/ws/tag-stream-client.ts` is the single transport/client:

- Maintains one socket instance (including `CONNECTING` reuse protection)
- Correlates responses by `cmd_id`
- Manages reconnect/backoff state
- Provides request helpers (`listChildren`, `addItem`, `removeItems`)
- Performs 2s polling for tracked IDs with `GET`

`src/lib/features/realtime/page-tag-realtime-provider.ts` wraps this client for route consumption and adds:

- active/inactive gating (poll only in Play mode)
- deduplicated desired ID tracking

## Tree Subsystem

### Data and state

- Data source adapter: `src/lib/features/tree/server-adapter.ts`
  - Fetches children with `tagStreamClient.listChildren(...)`
- Store: `src/lib/features/tree/tree-store.ts`
  - normalized node cache
  - expand/collapse
  - row flattening
  - branch refresh support (`refreshNode`)

### UI

- Main tree component: `src/lib/features/tree/components/VariableTree.svelte`
- Context menu system: `ContextMenu.svelte` + resolver contracts in `context-menu.ts`
- Add dialog:
  - opened from toolbar or folder context menu
  - supports root or folder parent target
  - fields: name, node type, and variable data-type/default value behavior
  - submit triggers `onCreateItem` (mapped in page route to WS `ADD`)

## Graph Subsystem

### Node model

`+page.svelte` creates `plantAsset` nodes that render via:

- `src/lib/features/graph/components/PlantAssetNode.svelte`
- dynamic resolution from `src/lib/features/graph/assets/registry.ts`

Asset metadata type is defined in `src/lib/features/graph/assets/types.ts`.

### Assets

Implemented assets include:

- Tank, Pump, Valve, Fan, Label, Slider, On/Off Input, Light Indicator, Typed Input

Shared wrapper:

- `src/lib/features/graph/assets/components/BaseAssetShell.svelte`

Typed input asset:

- `src/lib/features/graph/assets/components/TypedInputAsset.svelte`
- infers HTML input strategy from `sourceNode.dataType`
  - `Text` -> `type=text`
  - `Integer` -> `type=number`, `step=1`, `min=0`
  - `Float` -> `type=number`, `step=0.01`, `min=0`
- write behavior:
  - debounce 300ms while typing
  - immediate commit on Enter/blur
  - wheel increment prevention on numeric fields

## Page Orchestration (`+page.svelte`)

The route coordinates feature modules:

- starts/stops realtime provider on mount/unmount
- toggles polling by canvas mode (`edit` vs `play`)
- maps graph-tracked IDs to realtime desired IDs
- pushes live values into graph nodes through `live-utils.ts`
- wires tree context actions to WS commands
- wires drop menu to asset factory
- handles graph delete of selected nodes/edges

## Canvas Interaction Rules

Configured in `SvelteFlow` usage within `+page.svelte`:

- `selectionOnDrag` enabled for lasso selection in edit mode
- `panOnDrag={[1]}` for middle-mouse canvas pan
- `zoomOnDoubleClick={false}` to prevent accidental zoom from rapid clicks
- nodes/elements connect/select/drag only in edit mode

## Theme and UI State

- Theme mode store in `+page.svelte`
- CSS variable generation: `src/lib/core/theme/theme-utils.ts`
- Top controls in `src/lib/features/workspace/components/PageToolbar.svelte`

## Testing

Current automated tests cover core subsystems:

- `src/lib/core/ws/tag-stream-client.spec.ts`
- `src/lib/features/realtime/page-tag-realtime-provider.spec.ts`
- `src/lib/features/tree/server-adapter.spec.ts`
- `src/lib/features/tree/tree-store.spec.ts`
- `src/lib/features/tree/flatten.spec.ts`

## Known Intentional Constraints

- Realtime is polling-based (2s `GET`) until backend subscription support is introduced
- Tree and realtime commands intentionally share one WS transport to avoid duplicate socket handshakes
- Frontend may run standalone (`npm run dev`) or be launched by root Python orchestration in integrated runtime

# SCADA Svelte App — Implementation Architecture Outline

## Stack & Framework

- SvelteKit as the primary application framework.
- Use the current official SvelteKit architecture and conventions.
- Use TypeScript in strict mode across the codebase.
- Prefer clean, idiomatic Svelte components and SvelteKit routing.
- Keep the project modular, scalable, and suitable for an industrial SCADA-style application.
- Prefer server-driven and progressively enhanced patterns where possible.
- The application should support both:
  - public/documentation-style SSR pages if needed
  - authenticated app-style operational screens for the SCADA UI
- Use a structure that cleanly separates:
  - app shell
  - feature modules
  - domain models
  - API/client adapters
  - shared UI components
  - state management
  - tree browser subsystem

---

## Rendering Strategy

- Use SvelteKit SSR where it makes sense, especially for:
  - public routes
  - login/auth shell
  - documentation/help pages
  - initial app shell rendering
- For highly interactive SCADA operational views, use SSR for shell/layout and hydrate on the client for live interaction.
- Avoid hydration mismatches:
  - no random IDs generated during render unless deterministic
  - no time-based server/client differences in templates
  - no browser-only globals during SSR
- Guard browser-only APIs with SvelteKit/browser checks.
- Use load functions appropriately:
  - server load for SSR-safe initial data
  - universal/client load only when required
- Avoid duplicate fetching between server and client.
- Keep route-level code splitting enabled.
- Lazy load heavy operational modules where appropriate.
- Defer non-essential client-only features until after main UI becomes interactive.
- Optimize for Core Web Vitals even though this is an app:
  - keep initial shell light
  - minimize blocking scripts
  - lazy load heavy panels/views
  - avoid excessive client-side boot cost

---

## App Architecture

- Organize the app by domain and feature, not by file type alone.
- Suggested high-level structure:
  - `src/lib/core`
    - app configuration
    - environment handling
    - API clients
    - auth/session
    - logging
    - error normalization
  - `src/lib/domain`
    - SCADA entities and domain models
    - variable/node types
    - alarms/events models
    - historian/trend models
  - `src/lib/features`
    - tree browser
    - variable inspector
    - trends
    - alarms/events
    - dashboards
    - command panels
  - `src/lib/components`
    - reusable presentational UI components
  - `src/lib/stores`
    - shared app stores
  - `src/routes`
    - SvelteKit routes/layouts/pages
  - `src/lib/server`
    - server-only adapters and backend integration helpers where needed

- Separate clearly:
  - presentational components
  - controller/store logic
  - backend adapter logic
  - feature-specific domain logic

---

## SCADA UI Requirements

- The UI should feel like a professional industrial application, not a marketing website.
- Prioritize:
  - density
  - clarity
  - stable layouts
  - fast navigation
  - predictable interactions
- Typical application areas may include:
  - variable browser / namespace tree
  - live values view
  - alarm/event tables
  - trends/charts
  - command/action panels
  - diagnostics/status panels
- Favor desktop-style interaction patterns over oversized mobile-card layouts unless a mobile mode is explicitly required.

---

## Tree Structure Implementation

- Implement a high-performance explorer-style variable tree for browsing a tree-structured server namespace.
- This is not a decorative recursive tree widget.
- It should behave like:
  - OPC UA browser
  - SCADA tag browser
  - database/schema explorer
  - IDE/file explorer

### Tree UX Requirements

- One row per node.
- Disclosure chevron for expandable nodes.
- Indentation by depth.
- Clear icons for branch/object/folder/variable/leaf.
- Selection highlight on click.
- Async loading indicator per row/node.
- Expand/collapse behavior must feel instant after first load.
- Support large, deep, and wide server trees.
- Optional columns for:
  - name
  - value
  - data type
  - quality
  - timestamp

### Tree Architecture

- Do not render the final tree as deeply nested recursive DOM.
- Use a normalized tree cache plus a flattened visible-row model.
- Maintain:
  - canonical node cache
  - expanded node state
  - selected node state
  - loading state
  - per-node error state
  - derived visible rows list
- Render the UI from the flat visible rows list.

### Tree Data Model

- Each node must have a stable unique ID.
- Suggested node shape:
  - `id`
  - `parentId`
  - `name`
  - `path`
  - `kind`
  - `hasChildren`
  - `childIds | null`
  - optional metadata:
    - `value`
    - `dataType`
    - `quality`
    - `timestamp`
    - `raw`

### Tree State Model

- Centralized store should track:
  - `nodes`
  - `rootIds`
  - `expanded`
  - `selectedId`
  - `loading`
  - `errored`
  - optional `refreshing`

### Tree Loading Strategy

- Load root nodes initially.
- Fetch only immediate children when a node is expanded.
- Do not fetch full descendants.
- Cache children after first load.
- Re-expanding a loaded node should be instant.
- Allow optional manual refresh of node children.

### Tree Rendering Strategy

- Use a row-based tree grid approach.
- First column contains:
  - indentation
  - chevron
  - icon
  - node name
- Additional columns may show metadata.
- Keep row height fixed if possible.

### Tree Performance Strategy

- Build the architecture to be virtualization-ready from day one.
- If visible rows become large, integrate virtualization.
- Prefer flat rows + fixed row height.
- Avoid heavy expand/collapse animations.
- Avoid local row-level fetch ownership.

### Tree Keyboard Support

- Support standard explorer behavior:
  - Up / Down = previous/next row
  - Right = expand or move into child
  - Left = collapse or move to parent
  - Enter = inspect/open/select
  - Home / End = jump to first/last row

### Tree Accessibility

- Use appropriate tree or treegrid semantics.
- Include:
  - `aria-expanded`
  - `aria-selected`
  - `aria-level`
- Ensure full keyboard navigability.

### Tree Suggested Subsystem Structure

- `src/lib/features/tree/`
  - `types.ts`
  - `tree-store.ts`
  - `tree-controller.ts`
  - `flatten.ts`
  - `server-adapter.ts`
  - `components/`
    - `VariableTree.svelte`
    - `TreeHeader.svelte`
    - `TreeViewport.svelte`
    - `TreeRow.svelte`
    - `TreeChevron.svelte`
    - `TreeIcon.svelte`

---

## State Management

- Use Svelte stores for shared feature/application state.
- Keep store logic explicit and predictable.
- Prefer feature-scoped stores over one oversized global store.
- Use derived stores for computed UI state such as:
  - visible tree rows
  - selected node details
  - filtered alarm sets
  - derived trend selections
- Separate read models from command/action APIs where practical.
- Avoid pushing networking logic deep into presentational components.

---

## Data Access & Backend Integration

- Use a dedicated adapter/client layer for backend communication.
- Do not couple Svelte components directly to raw backend payloads.
- Normalize backend responses into frontend domain models.
- Keep integration points clean for protocols such as:
  - REST
  - WebSocket
  - SSE
  - OPC UA proxy backend
  - MQTT-backed backend APIs
- For live-updating data:
  - prefer patch/update flows
  - avoid rebuilding large UI structures on every update
- Handle:
  - reconnect states
  - stale data states
  - partial failure states
  - timeout/error normalization

---

## Routing & Layouts

- Use SvelteKit layouts to define:
  - public layout
  - auth layout
  - main application shell
- Typical application shell may include:
  - top bar
  - left navigation
  - central workspace
  - optional right-side inspector/details panel
  - optional bottom panel for logs/events
- Use route grouping cleanly for:
  - public pages
  - login/auth pages
  - app pages
- Keep heavy operational sections route-split where beneficial.

---

## SEO Requirements

- For public/documentation-facing routes:
  - implement unique metadata per route
  - title
  - description
  - canonical URL
  - Open Graph metadata
  - Twitter card metadata
- Generate:
  - `sitemap.xml`
  - `robots.txt`
- Use semantic HTML for public pages.
- Ensure SSR returns meaningful HTML for public routes.
- For authenticated SCADA operational pages, SEO is secondary, but metadata should still be sane and consistent.

---

## Styling

- Prefer TailwindCSS for layout and styling.
- Establish a restrained industrial design system.
- Define:
  - typography scale
  - spacing scale
  - breakpoints
  - color tokens
  - status colors
  - z-index conventions
- Prefer utility classes over large custom CSS files.
- Keep custom CSS limited to cases where utilities are insufficient.
- Design principles:
  - compact density
  - clear data hierarchy
  - strong readability
  - stable spacing
  - subtle hover/focus states
  - minimal decorative effects
- Avoid overly flashy transitions or heavy visual gimmicks.

---

## Design System Guidance

- Establish reusable primitives for:
  - panel/container
  - toolbar
  - table/grid
  - tree row
  - badge/status chip
  - tabs
  - dialogs
  - form controls
  - split panes
- Define consistent states for:
  - loading
  - empty
  - success
  - warning
  - error
  - stale/disconnected
- Keep visual behavior consistent across all SCADA modules.

---

## Tables, Grids, and Operational Views

- Use performant tabular rendering for:
  - alarms/events
  - variable lists
  - trends metadata
  - diagnostics
- Prefer architecture that supports:
  - sorting
  - filtering
  - pagination or virtualization
  - row selection
- Keep dense operational data views readable and keyboard-friendly.

---

## Charts / Trends

- Trends should be implemented as a separate feature area.
- Keep chart integration lightweight and purposeful.
- Avoid shipping large charting dependencies unless justified.
- Prioritize:
  - large dataset handling
  - zoom/pan
  - live update friendliness
  - clear axes/legends/tooltips
- Isolate trend data adapters and chart rendering components cleanly.

---

## Forms

- For login, configuration, filters, commands, and any edit panels:
  - use idiomatic Svelte forms and progressive enhancement patterns
  - optionally use a lightweight validation library if needed
- Build forms with:
  - client-side validation
  - accessible error messages
  - clear loading/success/error states
  - reusable schema/config approach where beneficial
- Keep form definitions isolated for complex forms.
- Avoid unnecessarily heavy form dependencies unless there is a strong reason.

---

## Accessibility

- Target WCAG-friendly defaults.
- Ensure:
  - keyboard navigation
  - focus visibility
  - proper labels
  - proper aria usage where needed
  - acceptable color contrast
- Pay special attention to:
  - tree/treegrid semantics
  - table/grid accessibility
  - dialogs
  - forms
  - status messages
- Accessibility should be built into the architecture, not patched on later.

---

## Performance & Best Practices

- Use strict TypeScript settings.
- Avoid heavy dependencies unless clearly justified.
- Prefer fine-grained, localized reactivity.
- Avoid excessive store churn for high-frequency live data.
- Patch/update affected nodes or rows rather than rebuilding whole structures.
- Keep route-level and feature-level code splitting enabled.
- Avoid blocking animations.
- Minimize JS cost for non-essential effects.
- Prefer fixed heights and virtualization-ready designs for dense data-heavy views.
- Keep hydration cost under control for operational screens.

---

## Error Handling & Resilience

- Provide a normalized error model for backend/API failures.
- Handle gracefully:
  - failed tree node loads
  - network interruptions
  - disconnected live streams
  - stale values
  - partial feature failures
- Errors should be localized where possible.
- One failed node or panel should not break the entire app shell.
- Provide retry patterns for relevant operations.

---

## Testing Strategy

- Include:
  - unit tests for domain and store logic
  - unit tests for tree flattening and expansion logic
  - integration tests for feature workflows
  - component tests for critical UI interactions
- Priority test targets:
  - tree expand/collapse/load behavior
  - selection logic
  - keyboard navigation
  - backend adapter normalization
  - live update patching
  - route/layout behavior
- Add performance sanity tests for large visible tree datasets if feasible.

---

## Deployment Notes

- The application must be deployable with Docker.
- Keep environment configuration clean and documented.
- Support typical Node hosting/container hosting.
- Make runtime port configurable by environment variable.
- Keep build output predictable and production-friendly.

---

## Deployment Strategy (Docker)

- The application must be deployable using Docker.
- Provide a multi-stage Docker build to minimize final image size.

### Typical Build Flow

1. Install dependencies
2. Build the SvelteKit app for production
3. Copy only required production artifacts into final runtime image
4. Run the production server adapter

### Final Container Requirements

- Run the production SvelteKit server
- Listen on port `4000` by default, or configurable via environment variable
- Work in typical container orchestration environments

---

## Docker Artifacts

- The repository should include:
  - `Dockerfile`
  - `.dockerignore`
  - `package.json` scripts for dev/build/preview or production serve
  - optional `docker-compose.yml` for local testing

### Example Runtime Behavior

- `docker build -t scada-svelte-app .`
- `docker run -p 4000:4000 scada-svelte-app`

---

## Build & Runtime Scripts

- Include scripts for:

### Development

- `npm run dev`

### Production build

- `npm run build`

### Production preview / serve

- `npm run preview`

- If a custom Node adapter/server is used, include explicit production serve scripts and document them clearly.

---

## Suggested SvelteKit Technical Choices

- TailwindCSS for styling
- Node adapter for SSR deployment unless another adapter is explicitly required
- Strict TypeScript
- Feature-scoped stores
- Server/client adapter separation
- Progressive enhancement for forms/actions
- Virtualization-ready architecture for heavy data views
- Flat-row tree implementation for server namespace browsing

---

## Final Architectural Direction

- Build this as a serious, modular SvelteKit SCADA application.
- Prioritize:
  - operational clarity
  - predictable state transitions
  - large-tree scalability
  - live-data friendliness
  - maintainable feature boundaries
- The variable browser/tree subsystem must be treated as a first-class feature with a scalable architecture, not as a simple recursive UI demo.
