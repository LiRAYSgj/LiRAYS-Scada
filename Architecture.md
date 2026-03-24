# LiRAYS-Scada Architecture

## Overview

LiRAYS-Scada is a SCADA (Supervisory Control and Data Acquisition) system designed for industrial automation and monitoring. The system consists of a Rust-based backend server and a SvelteKit-based frontend, communicating through a WebSocket protocol with protobuf-based messages.

## System Architecture

### High-Level Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Frontend      │    │   Backend       │    │   Data Storage  │
│   (SvelteKit)   │    │   (Rust)        │    │   (Sled DB)     │
│                 │    │                 │    │                 │
│  ┌───────────┐  │    │  ┌───────────┐  │    │  ┌───────────┐  │
│  │   Web     │  │    │  │   Web     │  │    │  │   Data    │  │
│  │  Browser  │  │    │  │  Server   │  │    │  │  Storage  │  │
│  └───────────┘  │    │  └───────────┘  │    │  └───────────┘  │
│                 │    │                 │    │                 │
│  ┌───────────┐  │    │  ┌───────────┐  │    │  ┌───────────┐  │
│  │   UI      │  │    │  │   Proto   │  │    │  │   Sled    │  │
│  │ Components│  │    │  │   Proto   │  │    │  │   Database│  │
│  └───────────┘  │    │  └───────────┘  │    │  └───────────┘  │
│                 │    │                 │    │                 │
│  ┌───────────┐  │    │  ┌───────────┐  │    │  ┌───────────┐  │
│  │  WebSocket│  │    │  │  WebSocket│  │    │  │  Data     │  │
│  │   Client  │  │    │  │   Server  │  │    │  │  Access   │  │
│  └───────────┘  │    │  └───────────┘  │    │  └───────────┘  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                       │                       │
        └───────────────────────┼───────────────────────┘
                                │
                    ┌─────────────────┐
                    │   Protocol      │
                    │   (protobuf)    │
                    └─────────────────┘
```

## Backend Architecture (Rust)

### Core Components

#### Server Module (`src/rtdata/server`)
- **WebSocket Server**: Handles real-time communication with frontend clients
- **Command Processing**: Processes incoming commands from frontend
- **Event Broadcasting**: Distributes events to subscribed clients
- **Variable Management**: Manages variable states and values
- **Protocol Handling**: Implements protobuf-based communication protocol

#### HTTP Module (`src/rtdata/http`)
- **Static File Serving**: Serves the compiled frontend application
- **API Gateway**: Provides HTTP interface to the system
- **Frontend Delivery**: Serves the SPA (Single Page Application)

#### Data Storage
- **Sled Database**: Embedded key-value store for persisting data
- **Variable Storage**: Stores real-time variable values and metadata
- **Namespace Management**: Stores tree structure and configuration data

### Communication Protocol

The backend uses a WebSocket-based communication protocol with protobuf messages:
- **Command Messages**: Binary protobuf messages for efficient communication
- **Event Messages**: Real-time updates for variable changes and tree modifications
- **JSON Fallback**: Support for JSON commands for debugging and development

### Command Types
- `LIST`: Fetch tree children
- `GET`: Poll live values  
- `SET`: Write values
- `ADD`: Create nodes
- `ADD_BULK`: Bulk namespace template creation
- `DEL`: Delete nodes

## Frontend Architecture (SvelteKit)

### Framework Stack
- **Framework**: SvelteKit with Svelte 5 runes and TypeScript
- **Rendering**: SSR-capable app with client hydration
- **UI Library**: Tailwind CSS + route-level CSS variables
- **Graph Engine**: @xyflow/svelte for visualization

### Core Modules

#### Tree Subsystem (`src/lib/features/tree`)
- **Data Management**: Server adapter for fetching tree data
- **State Management**: Tree store with normalized cache and expand/collapse state
- **Selection Logic**: Multi-selection with propagation support
- **UI Components**: Variable tree, context menu, tree rows, chevrons, icons

#### Graph Subsystem (`src/lib/features/graph`)
- **Node Model**: Dynamic asset resolution from registry
- **Asset Types**: Tank, Pump, Valve, Fan, Label, Slider, On/Off Input, Light Indicator, Typed Input
- **UI Components**: Plant asset nodes, base asset shell, typed input asset with HTML input strategy

#### WebSocket Client (`src/lib/core/ws`)
- **Shared Client**: Single transport/client instance
- **Reconnect Logic**: Manages connection state and reconnection with backoff
- **Request Helpers**: Methods for all command types
- **Polling**: 2s polling for tracked IDs with GET command

### Data Flow
1. **Initialization**: Frontend connects to backend via WebSocket
2. **Data Fetching**: Tree data requested via LIST commands
3. **Real-time Updates**: Events streamed for variable changes
4. **User Interaction**: Commands sent for modifications (ADD, DEL, SET)
5. **State Management**: Global stores manage UI and application state

## Build System

### Makefile Targets
- `build`: Builds both frontend and backend components
- `frontend`: Installs npm dependencies, generates protobuf, builds frontend
- `backend`: Builds Rust backend and creates Debian package
- `dev`: Starts development servers for both frontend and backend
- `clean`: Cleans build artifacts
- `test`: Runs Rust tests

### Build Process
1. **Frontend**: Uses Node.js with npm, TypeScript, and Svelte
2. **Backend**: Uses Rust with Cargo, with protobuf code generation
3. **Protobuf Integration**: Generated code for command and event handling
4. **Packaging**: Debian package creation using debuild

## Data Flow Architecture

### Real-time Data Path
```
[Frontend] ←→ [WebSocket] ←→ [Rust Server] ←→ [Sled Database]
     ↑                             ↓
     └── [Tree Requests] ──┘     [Variable Updates]
```

### User Interaction Flow
1. User interacts with UI components (tree, graph, forms)
2. Frontend sends command via WebSocket to backend
3. Backend processes command and updates state
4. Backend broadcasts events to subscribed clients
5. Frontend receives events and updates UI

## Technology Stack

### Backend
- **Language**: Rust
- **Web Framework**: Tokio with async/await
- **WebSocket**: tokio-tungstenite
- **Database**: Sled (embedded key-value store)
- **Protobuf**: prost for serialization
- **HTTP**: Axum framework for static file serving

### Frontend
- **Framework**: SvelteKit with Svelte 5 runes
- **Language**: TypeScript
- **Build System**: npm with Vite
- **UI Components**: Tailwind CSS
- **Graph Visualization**: @xyflow/svelte
- **State Management**: Svelte stores and global state management

### Protobuf
- **Messages**: Generated from .proto files in `proto/` directory
- **Commands**: Binary serialization for efficient network communication
- **Events**: Real-time updates for system state changes

## Deployment Architecture

### Development Environment
- `make dev`: Starts both frontend dev server and Rust backend
- Frontend hot-reloading enabled
- Backend with debug logging

### Production Environment
- `make release`: Builds optimized release version
- Single binary with embedded frontend
- Debian package generation for deployment

### Containerization
- Docker-based deployment strategy
- Self-contained binary with static frontend
- Environment variable configuration for binding addresses and ports

## Security Considerations

### Communication
- WebSocket protocol for real-time communication
- Binary protobuf messages for efficiency and security
- JSON fallback for debugging only

### Data Protection
- Embedded database with local storage
- No external network dependencies for core functionality
- State management through WebSocket session

## Performance Characteristics

### Real-time Updates
- 2-second polling interval for variable values
- Event-driven architecture for efficient updates
- WebSocket connection reuse for performance

### Data Handling
- Protobuf serialization for efficient binary communication
- In-memory caching for frequently accessed data
- Sled database for fast key-value operations

### Scalability
- Single-process architecture optimized for embedded use
- WebSocket connection handling with async/await
- Modular design for potential horizontal scaling

## Key Features

1. **Tree Browser**: Hierarchical navigation of system elements
2. **Real-time Monitoring**: Live data visualization and updates
3. **Graph Visualization**: Interactive system representation
4. **Variable Management**: Creation, modification, and deletion of system variables
5. **Namespace Templates**: Bulk creation of system elements
6. **Multi-user Support**: Shared WebSocket connections
7. **Responsive UI**: Adaptable interface for various devices
8. **Persistent Storage**: Data persistence through Sled database
