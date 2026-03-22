# LiRAYS Architecture Documentation

## Overview

LiRAYS (Lightweight Real-time Application System) is a real-time data management system designed for handling hierarchical data structures with efficient storage and communication. The system consists of both a backend implemented in Rust and a frontend built with SvelteKit, with Python components for integration and utilities.

## System Components

### 1. Core Architecture

The system follows a modular architecture with clear separation of concerns:

#### Backend (Rust)
- **Main Entry Point**: `src/lib.rs` - Exposes the Rust module to Python via PyO3
- **Server Layer**: `src/rtdata/server.rs` - WebSocket server handling client connections
- **Data Management**: `src/rtdata/variable.rs` - Core data manipulation and storage logic
- **Protocol Buffers**: Generated code from `.proto` files for message serialization
- **Utilities**: `src/rtdata/utils.rs` - Helper functions for path handling, type conversion, etc.

#### Frontend (SvelteKit)
- **Framework**: SvelteKit + Svelte 5 runes + TypeScript
- **Rendering**: SSR-capable app with client hydration for interactive SCADA screens
- **Main route**: `src/routes/+page.svelte`
- **Styling**: Tailwind CSS + route-level CSS variables (`src/routes/layout.css`)
- **Graph engine**: `@xyflow/svelte`
- **Transport**: single shared WebSocket client with command/response envelopes

#### Python Components
- **Integration Layer**: Python modules in `src/scada/` directory
- **Rust Integration**: `rustmod.pyi` and `rustmod.cpython-314-darwin.so` for calling Rust functions from Python
- **Data Management**: `data_dir.py` for handling data directory structure
- **LMDB CLI Tool**: `lmdb_cli.py` for inspecting LMDB databases
- **Protocol Buffers**: Generated Python code in `src/scada/proto/namespace/`

### 2. Data Storage

#### Backend Storage Engine
- **Backend**: `sled` (embedded database) for Rust backend
- **Storage Format**: Key-value storage with hierarchical organization
- **Tree Structure**: Single tree named "mainTree" with keys following specific patterns:
  - `H:{parent_path}\0{item_name}` - Hierarchical reference for items
  - `D:{item_path}` - Data storage for variable values

#### Data Models
- **Items**: Represented as `ItemMeta` records in the hierarchical tree
- **Variables**: Special items with `ItemType::Variable` that store data in `D:` keys
- **Folders**: Special items with `ItemType::Folder` used for organization

#### Python Data Management
- **Data Directory**: `data_dir.py` manages the data directory structure with `data_dir` and `rt_dir` paths
- **LMDB Integration**: `lmdb_cli.py` provides utilities for inspecting LMDB databases used in data storage

### 3. Communication Protocol

#### Backend Protocol

#### Message Format
- **Binary Protocol**: Based on Protocol Buffers for efficient serialization
- **JSON Protocol**: Text-based fallback for human-readable operations
- **WebSocket Transport**: Real-time communication channel between client and server

#### Message Types
- **Commands**: Client requests (ADD, LIST, SET, GET, DEL, ADD_BULK, SUB, UNSUB)
- **Responses**: Server replies to commands with status and results
- **Events**: Server-initiated notifications (variable value changes, tree modifications)

#### Frontend Protocol

#### WebSocket Architecture
The frontend uses command envelopes defined in `src/lib/core/ws/types.ts` and built in `command-ws-client.ts`:

- `LIST` for tree children fetch (optional `folderId`; undefined for roots)
- `GET` for polling live values
- `SET` for writing values
- `ADD` for creating nodes (`parentId`; empty string for root)
- `ADD_BULK` for namespace template (YAML) bulk create under a parent
- `DEL` for deleting one or more nodes by id

All commands use a global timeout (e.g. 60s); success/failure is determined by response `status` and optional `error_msg`. Command builders live in `src/lib/core/ws/command-ws-client.ts`.

#### Shared Client
`src/lib/core/ws/tag-stream-client.ts` is the single transport/client:

- Maintains one socket instance (including `CONNECTING` reuse protection)
- Correlates responses by `cmd_id`
- Manages reconnect/backoff state
- Provides request helpers (`listChildren`, `addItem`, `addBulkNamespace`, `removeItems`, etc.)
- Performs 2s polling for tracked IDs with `GET`
- Surfaces errors via a global snackbar store on timeout or non-OK status

### 4. Client Connection Handling

#### Backend Architecture
- **Multi-threaded**: Uses `tokio::spawn` to handle concurrent client connections
- **Asynchronous**: Fully non-blocking I/O using Tokio runtime
- **Broadcast Channel**: `tokio::sync::broadcast` for efficient event distribution to subscribed clients

#### Frontend Architecture
- **Single WebSocket Client**: `src/lib/core/ws/tag-stream-client.ts` maintains a single WebSocket connection
- **Reconnect Handling**: Automatic reconnection with backoff strategy
- **Command Correlation**: Responses are correlated by `cmd_id`

#### Connection Lifecycle
1. **Acceptance**: New TCP connections are accepted and upgraded to WebSocket
2. **Message Processing**: Commands are parsed and executed asynchronously
3. **Event Subscription**: Clients can subscribe to variable value changes or tree modifications
4. **Cleanup**: Automatic cleanup on client disconnection

### 5. Data Flow

#### Backend Data Flow

#### Add Operations
1. Client sends ADD command with parent folder and items metadata
2. Server validates parent exists and items don't already exist
3. Items are created in hierarchical structure (`H:` keys)
4. If successful, event is broadcasted to subscribed clients

#### Bulk Add Operations
1. Client sends ADD_BULK command with schema definition
2. Server recursively processes the schema tree
3. Items are created in batch for performance
4. Event is broadcasted upon successful batch application

#### Variable Operations
1. **SET**: Updates variable values (stored in `D:` keys)
2. **GET**: Retrieves variable values (from `D:` keys)
3. **SUBSCRIBE**: Client requests to receive events for specific variables
4. **UNSUBSCRIBE**: Client stops receiving events for specific variables

#### Delete Operations
1. Client sends DEL command with item IDs
2. Server removes items from both hierarchical tree and value storage
3. Children items are recursively removed
4. Event is broadcasted to notify of tree changes

#### Frontend Data Flow

#### Tree Subsystem
- Data source adapter: `src/lib/features/tree/server-adapter.ts`
  -
 Fetches children with `tagStreamClient.listChildren(parent?.id)`; `null` parent for roots
  - Node `id` and `path` come from backend (path-as-id from root ancestor to node)
- Store: `src/lib/features/tree/tree-store.ts`
  - normalized node cache
  - expand/collapse
  - row flattening
  - branch refresh support (`refreshNode`)
- Multi-selection: `src/lib/features/tree/tree-selection.ts`
  - `getLoadedDescendantIds(nodeId, nodes)` for propagation
  - `hasPartialSelectionInSubtree(nodeId, nodes, selection)` for indeterminate state
  - `getMinimalAncestorSet(selection, nodes, rootId)` for delete: returns only superior/fully-selected nodes (roots can be removed; partially selected nodes are not sent)

#### Graph Subsystem
- Node model: `src/lib/features/graph/components/PlantAssetNode.svelte` with dynamic resolution from `src/lib/features/graph/assets/registry.ts`
- Assets include: Tank, Pump, Valve, Fan, Label, Slider, On/Off Input, Light Indicator, Typed Input
- Typed input asset handles different HTML input strategies based on data type

### 6. Performance Optimizations

#### Backend Performance Optimizations
#### Memory Management
- **Pre-allocation**: Vectors are pre-allocated with appropriate capacity
- **String Handling**: Efficient string operations with proper capacity management
- **Batch Operations**: All data modifications use batch operations for performance

#### I/O Efficiency
- **Async Operations**: All file and network operations are asynchronous
- **Streaming**: Use of `scan_prefix` for efficient prefix-based tree scanning
- **Minimal Copying**: Reuse of existing data structures where possible

#### Data Structure Optimizations
- **HashSet for Existence Checks**: Used in `add_items` for fast duplicate detection
- **Broadcast Channel**: Efficient distribution of events to multiple subscribers
- **Efficient Path Handling**: Optimized path normalization and parsing

#### Frontend Performance Optimizations
- **SSR/Client Hydration**: SSR for shell/layout and client hydration for interactive components
- **Code Splitting**: Route-level code splitting enabled
- **Lazy Loading**: Heavy modules loaded on-demand
- **Optimized Updates**: Patch/update flows for live data instead of rebuilding UI structures

### 7. Error Handling

#### Backend Error Handling
#### Error Categories
- **Validation Errors**: Input validation, item existence checks
- **Storage Errors**: Database read/write failures
- **Protocol Errors**: Message parsing failures
- **Runtime Errors**: Unexpected internal errors

#### Error Propagation
- All errors are returned as `String` and wrapped with descriptive context
- Errors are logged appropriately using the `log` crate
- Client receives detailed error messages in responses

#### Frontend Error Handling
- **Reconnect States**: Handle network disconnections gracefully
-
 **Stale Data States**: Manage data consistency when connections are lost
- **Partial Failure States**: Handle failures in specific operations without crashing the entire app
- **Timeout/Error Normalization**: Consistent error presentation across the UI

### 8. Security Considerations

#### Backend Security
#### Input Validation
- All inputs are validated before processing
- Path normalization prevents directory traversal
- Type safety in Protocol Buffers ensures correct data formats

#### Access Control
- No built-in authentication/authorization (assumes trusted environment)
- Client identity determined by connection

#### Frontend Security
- Client-side validation only (no security enforcement)
- Secure WebSocket transport (wss:// if available)
- Proper error message handling to avoid information disclosure

### 9. Deployment Considerations

#### Backend Dependencies
- **Rust 1.70+**: Required for compilation
- **Tokio**: Async runtime
- **Sled**: Embedded database
- **Prost**: Protocol Buffer implementation
- **PyO3**: Python integration

#### Frontend Dependencies
- **SvelteKit**: Application framework
- **TypeScript**: Strong typing
- **Tailwind CSS**: Styling
- **@xyflow/svelte**: Graph rendering

#### Python Dependencies
- **lmdb**: LMDB database support
- **protobuf**: Protocol buffer support
- **PyO3**: Rust-Python integration

#### Performance
- **Backend**: Single-threaded for client connections (uses async model for I/O)
- **Frontend**: Optimized for Core Web Vitals
- **Memory usage**: Scales with data set size
- **Network throughput**: Depends on hardware and network conditions

#### Scalability
- **Backend**: Handles single-threaded I/O efficiently
- **Backend**: Can be scaled by running multiple instances behind a load balancer
- **Database performance**: May become a bottleneck for large datasets
- **Frontend**: Scales with client-side rendering capabilities

### 10. Key Design Patterns

#### Backend Design Patterns
#### Command Pattern
- Commands are parsed and executed using a match statement
- Each command type has dedicated handler function

#### Observer Pattern
- Events are broadcasted using `tokio::sync::broadcast` channel
- Clients subscribe to receive notifications about changes

#### Factory Pattern
- `ItemMeta` records created based on `ItemType` enum
- Variable data type conversion handled by dedicated functions

#### Resource Management
- Automatic cleanup of database connections
- Proper handling of batch operations for atomicity

#### Frontend Design Patterns
#### Component Architecture
- Modular Svelte components with clear responsibilities
- Reusable UI components and features

#### State Management
- Svelte stores for state management
- Centralized tree state management
- Real-time data tracking

#### Data Flow Pattern
- Separation of concerns between UI and data layers
- Adapter pattern for backend integration
- Event-driven updates for real-time data

### 11. Integration with Python Components

#### Rust-Python Integration
The Rust backend is exposed to Python through PyO3, allowing Python scripts and applications to leverage the core functionality.

#### Data Directory Management
Python utilities in `data_dir.py` manage the data directory structure, creating necessary directories for data storage.

#### LMDB Database Inspection
The `lmdb_cli.py` provides command-line tools for inspecting LMDB databases, useful for debugging and data management.

#### Protocol Buffer Integration
Python protocol buffer definitions in `src/scada/proto/namespace/` provide cross-language compatibility for data serialization.

### 12. Docker Deployment

#### Backend Container
- Rust-based container with all required dependencies
- Uses `tokio` runtime for async operations
- Embedded `sled` database for data storage

#### Frontend Container
- SvelteKit-based container with production build
- Server-side rendering capabilities
- Optimized for performance and security

#### Python Tools Container
- Contains Python dependencies for data management and CLI tools
- Includes `lmdb` and protobuf libraries
- Integration with the main system through shared data directories

### 13. Development and Build Process

#### Backend Build Process
- Rust compilation with `cargo`
- Protocol buffer compilation using `prost`
- PyO3 integration for Python binding

#### Frontend Build Process
- SvelteKit build with `npm run build`
- TypeScript compilation
- Tailwind CSS processing
- Code splitting and optimization

#### Python Build Process
- Module compilation for Python integration
- Protocol buffer generation
- CLI tool setup

#### Development Flow
1. Develop frontend in SvelteKit with TypeScript
2. Implement backend logic in Rust with Tokio
3. Integrate Python components for data management and utilities
4. Containerize both components for deployment
5. Test end-to-end integration between all components