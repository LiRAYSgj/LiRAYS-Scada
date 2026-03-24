FRONTEND_DIR=frontend
PROTO_DIR=proto
TARGET_DIR=target

# Detecta nvm
NVM_DIR=$(HOME)/.nvm

.PHONY: all
all: build

.PHONY: build
build: frontend backend

.PHONY: frontend
frontend:
	@echo "🔧 Building frontend..."
	@bash -c ' \
	export NVM_DIR="$(NVM_DIR)"; \
	[ -s "$$NVM_DIR/nvm.sh" ] && . "$$NVM_DIR/nvm.sh"; \
	nvm use 24; \
	cd $(FRONTEND_DIR); \
	if [ ! -d node_modules ]; then \
		echo "📦 Installing npm dependencies..."; \
		npm install; \
	fi; \
	echo "⚙️ Generating proto..."; \
	npm run generate:proto; \
	echo "🏗️ Building frontend..."; \
	npm run build; \
	'

.PHONY: backend
backend:
	@echo "🦀 Building Rust backend..."
	cargo build
	@cp target/debug/lirays-scada deb-files/usr/bin/

.PHONY: release
release: frontend
	@echo "🚀 Building release..."
	cargo build --release
	@cp target/release/lirays-scada deb-files/usr/bin/

.PHONY: clean
clean:
	@echo "🧹 Cleaning Rust..."
	cargo clean
	@echo "🧹 Cleaning frontend build..."
	rm -rf $(FRONTEND_DIR)/dist
	rm -rf $(FRONTEND_DIR)/build
	rm -rf $(FRONTEND_DIR)/node_modules

.PHONY: rebuild
rebuild: clean build

.PHONY: dev
dev:
	@echo "⚡ Starting dev mode..."
	@bash -c ' \
	export NVM_DIR="$(NVM_DIR)"; \
	[ -s "$$NVM_DIR/nvm.sh" ] && . "$$NVM_DIR/nvm.sh"; \
	nvm use 24; \
	cd $(FRONTEND_DIR) && npm run dev & \
	'
	cargo run

.PHONY: check
check:
	cargo check

.PHONY: test
test:
	cargo test
