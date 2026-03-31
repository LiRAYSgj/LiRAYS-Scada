FRONTEND_DIR=frontend
PROTO_DIR=proto
TARGET_DIR=target
NVM_DIR=$(HOME)/.nvm
UNAME_S := $(shell uname -s)
ARCH    := $(shell uname -m)
VERSION ?= 0.1.0

# Target por defecto según SO: mac -> tarball brew; linux -> .deb
ifeq ($(UNAME_S),Darwin)
DEFAULT_TARGET := mac
else
DEFAULT_TARGET := deb
endif

.PHONY: all
all: $(DEFAULT_TARGET)

.PHONY: mac
mac: frontend mac-tarball

.PHONY: deb
deb: frontend deb-package

.PHONY: frontend
frontend:
	@echo "🔧 Building frontend..."
	@bash -c ' \
	export NVM_DIR="$(NVM_DIR)"; \
	if [ -s "$$NVM_DIR/nvm.sh" ]; then . "$$NVM_DIR/nvm.sh"; nvm install 24 >/dev/null; nvm use 24; else echo "⚠️ nvm no encontrado, usando node del sistema ($$(node -v 2>/dev/null || echo missing))"; fi; \
	cd $(FRONTEND_DIR); \
	rm -rf node_modules \
	echo "📦 Installing npm dependencies..."; \
	npm install; \
	echo "⚙️ Generating proto..."; \
	npm run generate:proto; \
	echo "🏗️ Building frontend..."; \
	npm run build; \
	'

.PHONY: backend-build
backend-build: frontend
	@echo "🦀 Building Rust backend..."
	cargo build --release

.PHONY: deb-package
deb-package: backend-build
	@echo "📦 Creating Debian package..."
	cp target/release/lirays-scada deb-files/usr/bin/
	debuild -b -us -uc
	rm -rf ../*.build ../*.buildinfo ../*.changes ../*.ddeb
	mkdir -p distributions
	mv ../lirays-scada*.deb distributions

.PHONY: mac-tarball
mac-tarball: backend-build
	@echo "📦 Building macOS tarball for Homebrew..."
	VERSION=$(VERSION) packaging/homebrew/build_tarball.sh

.PHONY: clean
clean:
	@echo "🧹 Cleaning Rust..."
	cargo clean
	@echo "🧹 Cleaning frontend build..."
	rm -rf $(FRONTEND_DIR)/dist
	rm -rf $(FRONTEND_DIR)/build
	rm -rf $(FRONTEND_DIR)/node_modules

.PHONY: rebuild
rebuild: clean all

.PHONY: test
test:
	@echo "Running tests..."
	@bash -c ' \
	export NVM_DIR="$(NVM_DIR)"; \
	if [ -s "$$NVM_DIR/nvm.sh" ]; then . "$$NVM_DIR/nvm.sh"; nvm install 24 >/dev/null; nvm use 24; else echo "⚠️ nvm no encontrado, usando node del sistema ($$(node -v 2>/dev/null || echo missing))"; fi; \
	cd $(FRONTEND_DIR); \
	npm run test; \
	cd ..; \
	'
	cargo test
