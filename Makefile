SHELL := /usr/bin/env bash

ROOT          := $(CURDIR)
FRONTEND_DIR  := $(ROOT)/frontend
TARGET_DIR    := $(ROOT)/target
DIST_DIR      := $(ROOT)/distributions
PACKAGING_DIR := $(ROOT)/packaging
DEB_ROOT_DIR  := $(PACKAGING_DIR)/debian
DEBIAN_DIR    := $(DEB_ROOT_DIR)/debian
DEBFILES_DIR  := $(DEB_ROOT_DIR)/deb-files
NVM_DIR       ?= $(HOME)/.nvm
NODE_VERSION  ?= 24
VERSION       ?= $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH          ?= $(shell uname -m)
UNAME_S       := $(shell uname -s)
OS            := $(shell uname -s)
NSSM_PATH     ?= $(ROOT)/packaging/windows/nssm.exe
DOCKER        ?= docker
CARGO         ?= cargo

# Host detection
ifeq ($(OS),Windows_NT)
  HOST_OS := windows
else ifneq (,$(findstring MINGW,$(UNAME_S)))
  HOST_OS := windows
else ifeq ($(UNAME_S),Darwin)
  HOST_OS := mac
else ifeq ($(UNAME_S),Linux)
  HOST_OS := linux
else
  HOST_OS := unknown
endif

ifeq ($(HOST_OS),mac)
  DEFAULT_TARGET := mac-local
else ifeq ($(HOST_OS),linux)
  DEFAULT_TARGET := deb-local
else ifeq ($(HOST_OS),windows)
  DEFAULT_TARGET := windows-local
else
  DEFAULT_TARGET := help
endif

FRONTEND_STAMP := $(FRONTEND_DIR)/.frontend-built

.PHONY: all help release clean frontend backend-build mac mac-local mac-package mac-dmg mac-all \
        deb deb-local deb-package deb-docker-amd64 deb-docker-arm64 \
        windows windows-local windows-build windows-package windows-cross \
        test rebuild \
        ensure-mac ensure-linux ensure-windows prepare-dist

all: $(DEFAULT_TARGET)

help:
	@echo "Targets:"
	@echo "  make                -> build installer for current OS/arch (no Docker)"
	@echo "  make release        -> full release from macOS: mac (arm/x86), deb (amd/arm), windows zip/NSIS"
	@echo "  make mac-local      -> DMG app bundle for host mac arch"
	@echo "  make mac-all        -> DMG app bundle for arm64 + x86_64 (mac host)"
	@echo "  make deb-local      -> .deb for host arch (Debian/Ubuntu)"
	@echo "  make windows-local  -> zip (+NSIS if makensis) for host Windows"
	@echo "  make clean          -> clean Rust + frontend outputs"

# --- Shared steps -----------------------------------------------------------

prepare-dist:
	@mkdir -p $(DIST_DIR)

$(FRONTEND_STAMP):
	@echo "🔧 Building frontend..."
	@bash -c 'set -e; \
		export NVM_DIR="$(NVM_DIR)"; \
		if [ -s "$$NVM_DIR/nvm.sh" ]; then \
		  . "$$NVM_DIR/nvm.sh"; nvm install $(NODE_VERSION) >/dev/null; nvm use $(NODE_VERSION); \
		else \
		  echo "⚠️  nvm not found, using system node ($$(node -v 2>/dev/null || echo missing))"; \
		fi; \
		cd "$(FRONTEND_DIR)"; \
		npm install; \
		npm run generate:proto; \
		npm run build; \
	'
	@touch "$@"

frontend: $(FRONTEND_STAMP)

backend-build: frontend
	@PATH="/usr/local/cargo/bin:/root/.cargo/bin:$$PATH" HOST_OS="$(HOST_OS)" BACKEND_TARGET="$(BACKEND_TARGET)" CARGO="$(CARGO)" bash -lc 'set -e; \
		if ! command -v "$$CARGO" >/dev/null 2>&1; then echo "cargo not found in PATH=$$PATH" >&2; exit 127; fi; \
		TARGET="$$BACKEND_TARGET"; \
		if [ -n "$$TARGET" ]; then rustup target add $$TARGET >/dev/null 2>&1 || true; fi; \
		CMD="$$CARGO build --release"; \
		if [ -n "$$TARGET" ]; then CMD="$$CMD --target $$TARGET"; fi; \
		if [ "$$HOST_OS" = "mac" ] && [[ "$$TARGET" == *"apple-darwin"* ]]; then \
		  SDKROOT="$$(xcrun --sdk macosx --show-sdk-path)" $$CMD; \
		else \
		  $$CMD; \
		fi'

clean:
	@echo "🧹 Cleaning Rust..." && $(CARGO) clean
	@echo "🧹 Cleaning frontend build..."
	@rm -rf $(FRONTEND_DIR)/dist $(FRONTEND_DIR)/build $(FRONTEND_DIR)/node_modules $(FRONTEND_STAMP)

rebuild: clean all

test:
	@echo "Running tests..."
	@bash -c 'set -e; \
		export NVM_DIR="$(NVM_DIR)"; \
		if [ -s "$$NVM_DIR/nvm.sh" ]; then \
		  . "$$NVM_DIR/nvm.sh"; nvm install $(NODE_VERSION) >/dev/null; nvm use $(NODE_VERSION); \
		else \
		  echo "⚠️  nvm not found, using system node ($$(node -v 2>/dev/null || echo missing))"; \
		fi; \
		cd "$(FRONTEND_DIR)"; \
		npm run test; \
	'
	@$(CARGO) test

# --- macOS ------------------------------------------------------------------

mac: mac-local

mac-package: backend-build mac-dmg

mac-dmg: prepare-dist
	@ARCH=$(ARCH) VERSION=$(VERSION) packaging/macos/build_dmg.sh

mac-local: ensure-mac
	@$(MAKE) ARCH=$(ARCH) BACKEND_TARGET= mac-package

mac-all: ensure-mac
	@$(MAKE) clean
	@$(MAKE) ARCH=arm64 BACKEND_TARGET= mac-package
	@$(MAKE) ARCH=x86_64 BACKEND_TARGET=x86_64-apple-darwin mac-package

# --- Debian/Ubuntu ----------------------------------------------------------

deb: deb-local

deb-local: ensure-linux backend-build deb-package

deb-package: prepare-dist
	@echo "📦 Creating Debian package..."
	@bash -lc 'set -euo pipefail; \
	  ROOT="$(ROOT)"; \
	  DIST="$(DIST_DIR)"; \
	  DEBFILES="$(DEBFILES_DIR)"; \
	  install -Dm644 "$$ROOT/settings-default.yaml" "$$DEBFILES/etc/lirays-scada/settings.yaml"; \
	  ln -sfn "$(DEBIAN_DIR)" "$$ROOT/debian"; \
	  ln -sfn "$(DEBFILES_DIR)" "$$ROOT/deb-files"; \
	  trap "rm -f \"$$ROOT/debian\" \"$$ROOT/deb-files\"" EXIT; \
	  install -Dm755 "$(TARGET_DIR)/release/lirays-scada" "$$DEBFILES/usr/bin/lirays-scada"; \
	  cd "$$ROOT"; \
	  DEB_BUILD_OPTIONS="nostrip nocheck" debuild -b -us -uc; \
	  rm -rf ../*.build ../*.buildinfo ../*.changes ../*.ddeb ../*dbgsym*.deb; \
	  mv ../lirays-scada_*[0-9].deb "$$DIST"; \
	'

# cross-build .deb from macOS via Docker

deb-docker-amd64: ensure-mac
	@echo "==> Building .deb (amd64) in Docker"
	@$(DOCKER) run --rm --platform=linux/amd64 -v $(ROOT):/src -w /src rust:1.94-bullseye bash -lc 'set -e; \
		apt-get update; \
		apt-get install -y curl ca-certificates gnupg; \
		curl -fsSL https://deb.nodesource.com/setup_$(NODE_VERSION).x | bash -; \
		apt-get install -y nodejs protobuf-compiler debhelper devscripts equivs pkg-config libssl-dev build-essential; \
		ln -sfn packaging/debian/debian ./debian; \
		ln -sfn packaging/debian/deb-files ./deb-files; \
		mk-build-deps -i -r -t "apt-get -y --no-install-recommends" ./debian/control; \
		export PATH="/usr/local/cargo/bin:/root/.cargo/bin:$$PATH"; \
		export CARGO=/usr/local/cargo/bin/cargo; \
		VERSION=$(VERSION) CARGO=$$CARGO make deb-local; \
		rm -f ./debian ./deb-files'


deb-docker-arm64: ensure-mac
	@echo "==> Building .deb (arm64) in Docker"
	@$(DOCKER) run --rm --platform=linux/arm64 -v $(ROOT):/src -w /src rust:1.94-bullseye bash -lc 'set -e; \
		apt-get update; \
		apt-get install -y curl ca-certificates gnupg; \
		curl -fsSL https://deb.nodesource.com/setup_$(NODE_VERSION).x | bash -; \
		apt-get install -y nodejs protobuf-compiler debhelper devscripts equivs pkg-config libssl-dev build-essential; \
		ln -sfn packaging/debian/debian ./debian; \
		ln -sfn packaging/debian/deb-files ./deb-files; \
		mk-build-deps -i -r -t "apt-get -y --no-install-recommends" ./debian/control; \
		export PATH="/usr/local/cargo/bin:/root/.cargo/bin:$$PATH"; \
		export CARGO=/usr/local/cargo/bin/cargo; \
		VERSION=$(VERSION) CARGO=$$CARGO make deb-local; \
		rm -f ./debian ./deb-files'

# --- Windows ---------------------------------------------------------------

windows: windows-local

WINDOWS_TARGET ?=
WINDOWS_ARCH   ?= $(if $(findstring 86,$(ARCH)),x86_64,$(ARCH))

windows-build:
	@$(MAKE) BACKEND_TARGET=$(WINDOWS_TARGET) backend-build

windows-package: prepare-dist
	@echo "📦 Packaging Windows $(WINDOWS_ARCH)"
	@BIN_PATH="$(TARGET_DIR)/release/lirays-scada.exe"; \
	if [ -n "$(WINDOWS_TARGET)" ]; then BIN_PATH="$(TARGET_DIR)/$(WINDOWS_TARGET)/release/lirays-scada.exe"; fi; \
	if [ ! -f "$$BIN_PATH" ]; then echo "Binary not found at $$BIN_PATH" >&2; exit 1; fi; \
	OUT_DIR="$(DIST_DIR)/windows-$(WINDOWS_ARCH)"; \
	rm -rf "$$OUT_DIR"; mkdir -p "$$OUT_DIR"; \
	cp "$$BIN_PATH" "$$OUT_DIR/"; \
	cp packaging/windows/install.ps1 packaging/windows/uninstall.ps1 "$$OUT_DIR/"; \
	if [ -f "$(NSSM_PATH)" ]; then cp "$(NSSM_PATH)" "$$OUT_DIR/"; else echo "⚠️  nssm.exe missing; zip will lack the service helper"; fi; \
	( cd $(DIST_DIR) && zip -r "lirays-scada-$(VERSION)-windows-$(WINDOWS_ARCH).zip" "windows-$(WINDOWS_ARCH)" ); \
	if command -v makensis >/dev/null 2>&1 && [ -f "$(NSSM_PATH)" ]; then \
	  cp packaging/windows/setup.nsi "$$OUT_DIR/"; \
	  (cd "$$OUT_DIR" && makensis -DVERSION="$(VERSION)" -DARCH="$(WINDOWS_ARCH)" setup.nsi); \
	  mv "$$OUT_DIR/LiRays-Scada-$(VERSION)-$(WINDOWS_ARCH)-Setup.exe" $(DIST_DIR)/; \
	else \
	  echo "ℹ️  makensis or nssm.exe missing; skipping NSIS installer."; \
	fi; \
	rm -rf "$$OUT_DIR"

windows-local: ensure-windows windows-build windows-package

windows-cross: ensure-mac
	@echo "==> Building Windows x86_64 via Docker (mingw)"
	@$(DOCKER) run --rm --platform=linux/amd64 -v $(ROOT):/src -w /src rust:1.94-bullseye bash -lc 'set -e; \
		apt-get update; \
		apt-get install -y curl ca-certificates gnupg; \
		curl -fsSL https://deb.nodesource.com/setup_$(NODE_VERSION).x | bash -; \
		apt-get install -y nodejs mingw-w64 zip protobuf-compiler pkg-config; \
		export PATH="/usr/local/cargo/bin:/root/.cargo/bin:$$PATH"; \
		export CARGO=/usr/local/cargo/bin/cargo; \
		rustup target add x86_64-pc-windows-gnu; \
		make frontend; \
		$$CARGO build --release --target x86_64-pc-windows-gnu'
	@$(MAKE) WINDOWS_TARGET=x86_64-pc-windows-gnu WINDOWS_ARCH=x86_64 windows-package

# --- Meta targets -----------------------------------------------------------

release: ensure-mac
	@echo "==> Full release (mac host)"
	@$(MAKE) mac-all
	@$(MAKE) deb-docker-amd64
	@$(MAKE) deb-docker-arm64
	@$(MAKE) windows-cross
	@echo "Artifacts ready in $(DIST_DIR)"

ensure-mac:
	@if [ "$(HOST_OS)" != "mac" ]; then echo "This target must run on macOS" >&2; exit 1; fi

ensure-linux:
	@if [ "$(HOST_OS)" != "linux" ]; then echo "This target must run on Debian/Ubuntu (Linux)" >&2; exit 1; fi

ensure-windows:
	@if [ "$(HOST_OS)" != "windows" ]; then echo "This target must run on Windows" >&2; exit 1; fi
