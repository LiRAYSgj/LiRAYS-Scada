SHELL := /usr/bin/env bash

ROOT                  := $(CURDIR)
FRONTEND_DIR          := $(ROOT)/frontend
TARGET_DIR            := $(ROOT)/target
DIST_DIR              := $(ROOT)/distributions
PACKAGING_DIR         := $(ROOT)/packaging
DEB_ROOT_DIR          := $(PACKAGING_DIR)/debian
DEBIAN_DIR            := $(DEB_ROOT_DIR)/debian
DEBFILES_DIR          := $(DEB_ROOT_DIR)/deb-files
RPM_DIR               := $(PACKAGING_DIR)/rpm
RPM_SPECS_DIR         := $(RPM_DIR)/SPECS
RPM_SOURCES_DIR       := $(RPM_DIR)/SOURCES
MAC_DIR               := $(PACKAGING_DIR)/macos
MAC_PKGROOT           := $(MAC_DIR)/pkgroot
MAC_SCRIPTS           := $(MAC_DIR)/scripts
MAC_PKG_OUT           := $(DIST_DIR)
MAC_WORK              := $(MAC_DIR)/work
MAC_IDENTIFIER        := com.lirays.liraysscada
NVM_DIR               ?= $(HOME)/.nvm
NODE_VERSION          ?= 24
FRONTEND_BUILD_NODE_OPTIONS ?= --max-old-space-size=6144
VERSION               ?= $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DOCKER                ?= docker
DOCKER_BUILDX         ?= $(DOCKER) buildx
CARGO                 ?= cargo
DOCKER_REPOSITORY     ?= lirays-scada
DOCKER_IMAGE_VERSION  ?= $(DOCKER_REPOSITORY):$(VERSION)
DOCKER_IMAGE_LATEST   ?= $(DOCKER_REPOSITORY):latest
DOCKER_PLATFORM_AMD64 ?= linux/amd64
DOCKER_PLATFORM_ARM64 ?= linux/arm64
DOCKER_IMAGE_VERSION_AMD64 ?= $(DOCKER_IMAGE_VERSION)-amd64
DOCKER_IMAGE_VERSION_ARM64 ?= $(DOCKER_IMAGE_VERSION)-arm64
DOCKER_IMAGE_LATEST_AMD64  ?= $(DOCKER_IMAGE_LATEST)-amd64
DOCKER_IMAGE_LATEST_ARM64  ?= $(DOCKER_IMAGE_LATEST)-arm64

FRONTEND_STAMP := $(FRONTEND_DIR)/build/.frontend-built-stamp

.PHONY: all help clean test frontend \
	linux docker docker-amd64 docker-arm64 publish \
	deb deb-amd64 deb-arm64 \
	rpm rpm-amd64 rpm-arm64 rpm-package \
	mac macos-build-x86 macos-build-arm macos-universal macos-stage macos-pkg \
	win

all: linux mac docker

help:
	@echo "Targets:"
	@echo "  make           -> build Linux packages + macOS package + Docker image"
	@echo "  make linux     -> build DEB and RPM packages (amd64+arm64 using Docker)"
	@echo "  make deb       -> build Debian packages (Docker, amd64+arm64)"
	@echo "  make rpm       -> build RPM packages (Docker, amd64+arm64)"
	@echo "  make mac       -> build macOS installer package (.pkg)"
	@echo "  make docker    -> build Docker images for amd64+arm64 (local arch-suffixed tags)"
	@echo "  make publish   -> push arch tags and publish multi-arch :$(VERSION) + :latest manifests"
	@echo "  make win       -> placeholder (prints not supported)"
	@echo "  make test      -> frontend tests + cargo test"
	@echo "  make clean     -> clean Rust + frontend outputs"

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
		NODE_OPTIONS="$${NODE_OPTIONS:-} $(FRONTEND_BUILD_NODE_OPTIONS)" npm run build; \
	'
	@test -d "$(FRONTEND_DIR)/build"
	@mkdir -p "$(dir $@)"
	@touch "$@"

frontend: $(FRONTEND_STAMP)

clean:
	@echo "🧹 Cleaning Rust..." && $(CARGO) clean
	@echo "🧹 Cleaning frontend build..."
	@rm -rf $(FRONTEND_DIR)/dist $(FRONTEND_DIR)/build $(FRONTEND_DIR)/node_modules $(FRONTEND_STAMP)
	@echo "🧹 Cleaning packaging artifacts..."
	@if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then \
		git clean -fdX $(PACKAGING_DIR); \
	else \
		echo "⚠️  Skipping packaging git-clean (not a git repository)."; \
	fi
	@echo "🧹 Cleaning distributions..."
	@rm -rf $(DIST_DIR)
	@rm -f *.build *.buildinfo *.changes *.ddeb *.deb *.rpm

test: $(FRONTEND_STAMP)
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

linux: deb rpm

# --- DEB (Debian/Ubuntu) ----------------------------------------------------------

deb: deb-amd64 deb-arm64

deb-package: prepare-dist
	@echo "📦 Creating Debian package..."
	@bash -lc 'set -euo pipefail; \
	  ROOT="$(ROOT)"; \
	  DIST="$(DIST_DIR)"; \
	  DEBFILES="$(DEBFILES_DIR)"; \
	  ln -sfn "$(DEBIAN_DIR)" "$$ROOT/debian"; \
	  ln -sfn "$(DEBFILES_DIR)" "$$ROOT/deb-files"; \
	  trap "rm -f \"$$ROOT/debian\" \"$$ROOT/deb-files\"" EXIT; \
	  install -Dm755 "$(TARGET_DIR)/release/lirays-scada" "$$DEBFILES/usr/bin/lirays-scada"; \
	  install -Dm755 "$(TARGET_DIR)/release/lirays" "$$DEBFILES/usr/bin/lirays"; \
	  cd "$$ROOT"; \
	  DEB_BUILD_OPTIONS="nostrip nocheck" debuild -b -us -uc; \
	  rm -rf ../*.build ../*.buildinfo ../*.changes ../*.ddeb ../*dbgsym*.deb; \
	  mv ../lirays-scada_*[0-9].deb "$$DIST"; \
	'

deb-amd64: prepare-dist $(FRONTEND_STAMP)
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
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release --bins; \
		VERSION=$(VERSION) CARGO=$$CARGO make deb-package; \
		rm -f ./debian ./deb-files'

deb-arm64: prepare-dist $(FRONTEND_STAMP)
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
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release --bins; \
		VERSION=$(VERSION) CARGO=$$CARGO make deb-package; \
		rm -f ./debian ./deb-files'

# --- RPM (RHEL/Rocky/Fedora) -----------------------------------------------------

rpm: rpm-amd64 rpm-arm64

rpm-package: prepare-dist
	@echo "📦 Creating RPM package..."
	@bash -lc 'set -euo pipefail; \
	  ROOT="$(ROOT)"; \
	  DIST="$(DIST_DIR)"; \
	  RPM_TOP="$(RPM_DIR)"; \
	  SOURCES="$(RPM_SOURCES_DIR)"; \
	  mkdir -p "$$RPM_TOP/BUILD" "$$RPM_TOP/BUILDROOT" "$$RPM_TOP/RPMS" "$$RPM_TOP/SOURCES" "$$RPM_TOP/SPECS" "$$RPM_TOP/SRPMS"; \
	  install -Dm755 "$(TARGET_DIR)/release/lirays-scada" "$$SOURCES/lirays-scada"; \
	  install -Dm755 "$(TARGET_DIR)/release/lirays" "$$SOURCES/lirays"; \
	  rpmbuild -bb "$$RPM_TOP/SPECS/lirays-scada.spec" \
	    --define "_topdir $$RPM_TOP" \
	    --define "version $(VERSION)"; \
	  find "$$RPM_TOP/RPMS" -maxdepth 2 -name "*.rpm" -print -exec mv {} "$$DIST" \; ; \
	  rm -f "$$SOURCES/lirays-scada"; \
	  rm -f "$$SOURCES/lirays"; \
	'

rpm-amd64: prepare-dist $(FRONTEND_STAMP)
	@echo "==> Building .rpm (amd64) in Docker"
	@$(DOCKER) run --rm --platform=linux/amd64 -v $(ROOT):/src -w /src rockylinux:9 bash -lc 'set -e; \
		dnf -y install dnf-plugins-core; \
		dnf config-manager --set-enabled crb; \
		dnf -y install epel-release; \
		curl -fsSL https://rpm.nodesource.com/setup_$(NODE_VERSION).x | bash -; \
		dnf -y install nodejs protobuf-compiler rpm-build rpmdevtools systemd-rpm-macros shadow-utils gcc gcc-c++ make pkgconfig openssl-devel git; \
		curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.94.0; \
		. "$$HOME/.cargo/env"; \
		export PATH="$$HOME/.cargo/bin:$$PATH"; \
		export CARGO=$$HOME/.cargo/bin/cargo; \
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release --bins; \
		VERSION=$(VERSION) CARGO=$$CARGO make rpm-package; \
	'

rpm-arm64: prepare-dist $(FRONTEND_STAMP)
	@echo "==> Building .rpm (arm64) in Docker"
	@$(DOCKER) run --rm --platform=linux/arm64 -v $(ROOT):/src -w /src rockylinux:9 bash -lc 'set -e; \
		dnf -y install dnf-plugins-core; \
		dnf config-manager --set-enabled crb; \
		dnf -y install epel-release; \
		curl -fsSL https://rpm.nodesource.com/setup_$(NODE_VERSION).x | bash -; \
		dnf -y install nodejs protobuf-compiler rpm-build rpmdevtools systemd-rpm-macros shadow-utils gcc gcc-c++ make pkgconfig openssl-devel git; \
		curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.94.0; \
		. "$$HOME/.cargo/env"; \
		export PATH="$$HOME/.cargo/bin:$$PATH"; \
		export CARGO=$$HOME/.cargo/bin/cargo; \
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release --bins; \
		VERSION=$(VERSION) CARGO=$$CARGO make rpm-package; \
	'

# --- macOS ----------------------------------------------------

mac: macos-pkg

macos-build-x86: $(FRONTEND_STAMP)
	@rustup target add x86_64-apple-darwin >/dev/null 2>&1 || true
	@echo "🏗️  Building macOS (x86_64)..."
	@$(CARGO) build --release --target x86_64-apple-darwin --bins

macos-build-arm: $(FRONTEND_STAMP)
	@rustup target add aarch64-apple-darwin >/dev/null 2>&1 || true
	@echo "🏗️  Building macOS (arm64)..."
	@$(CARGO) build --release --target aarch64-apple-darwin --bins

macos-universal: prepare-dist macos-build-x86 macos-build-arm
	@echo "🔗 Creating universal binary..."
	@mkdir -p $(MAC_WORK)
	@lipo -create -output $(MAC_WORK)/lirays_scada \
		$(TARGET_DIR)/x86_64-apple-darwin/release/lirays-scada \
		$(TARGET_DIR)/aarch64-apple-darwin/release/lirays-scada
	@chmod +x $(MAC_WORK)/lirays_scada
	@lipo -create -output $(MAC_WORK)/lirays \
		$(TARGET_DIR)/x86_64-apple-darwin/release/lirays \
		$(TARGET_DIR)/aarch64-apple-darwin/release/lirays
	@chmod +x $(MAC_WORK)/lirays

macos-stage: macos-universal
	@echo "📁 Staging macOS pkg root..."
	@bash -lc 'set -euo pipefail; \
	  PKGROOT="$(MAC_PKGROOT)"; \
	  APP_SUPPORT="$$PKGROOT/Library/Application Support/LiRAYSScada"; \
	  LAUNCHD="$$PKGROOT/Library/LaunchDaemons"; \
	  LIBEXEC="$$PKGROOT/usr/local/libexec/liraysscada"; \
	  BIN_DIR="$$PKGROOT/usr/local/bin"; \
	  SETTINGS_DEFAULT="$$APP_SUPPORT/settings.default.yaml"; \
	  if [ ! -f "$$SETTINGS_DEFAULT" ]; then \
	    echo "Missing $$APP_SUPPORT/settings.default.yaml. Add the macOS default settings file before running make."; \
	    exit 1; \
	  fi; \
	  rm -rf "$$LIBEXEC" "$$BIN_DIR" "$$LAUNCHD"; \
	  mkdir -p "$$LIBEXEC" "$$BIN_DIR" "$$APP_SUPPORT" "$$LAUNCHD"; \
	  install -m755 "$(MAC_WORK)/lirays_scada" "$$LIBEXEC/lirays_scada"; \
	  install -m755 "$(MAC_WORK)/lirays" "$$BIN_DIR/lirays"; \
	  install -m755 "$(MAC_DIR)/bin/lirays-uninstall" "$$BIN_DIR/lirays-uninstall"; \
	  install -m644 "$(MAC_DIR)/com.lirays.liraysscada.plist" "$$LAUNCHD/com.lirays.liraysscada.plist"; \
	'

macos-pkg: frontend macos-stage
	@echo "📦 Building macOS installer (.pkg)..."
	@bash -lc 'set -euo pipefail; \
	  OUTDIR="$(MAC_PKG_OUT)"; \
	  PKGROOT="$(MAC_PKGROOT)"; \
	  SCRIPTS="$(MAC_SCRIPTS)"; \
	  IDENT="$(MAC_IDENTIFIER)"; \
	  COMPONENT="$(MAC_WORK)/liraysscada-component.pkg"; \
	  FINAL="$$OUTDIR/LiRAYSScada-$(VERSION).pkg"; \
	  pkgbuild --root "$$PKGROOT" --identifier "$$IDENT" --version $(VERSION) --scripts "$$SCRIPTS" "$$COMPONENT"; \
	  productbuild --package "$$COMPONENT" "$$FINAL"; \
	  rm -f "$$COMPONENT"; \
	  rm -rf "$(MAC_WORK)"; \
	  echo "Built $$FINAL"; \
	'

docker: docker-amd64 docker-arm64

docker-amd64:
	@echo "🐳 Building Docker image tags ($(DOCKER_PLATFORM_AMD64)): $(DOCKER_IMAGE_VERSION_AMD64) and $(DOCKER_IMAGE_LATEST_AMD64)"
	@$(DOCKER_BUILDX) build --platform "$(DOCKER_PLATFORM_AMD64)" --target production --build-arg FRONTEND_BUILD_NODE_OPTIONS="$(FRONTEND_BUILD_NODE_OPTIONS)" -t "$(DOCKER_IMAGE_VERSION_AMD64)" -t "$(DOCKER_IMAGE_LATEST_AMD64)" --load .

docker-arm64:
	@echo "🐳 Building Docker image tags ($(DOCKER_PLATFORM_ARM64)): $(DOCKER_IMAGE_VERSION_ARM64) and $(DOCKER_IMAGE_LATEST_ARM64)"
	@$(DOCKER_BUILDX) build --platform "$(DOCKER_PLATFORM_ARM64)" --target production --build-arg FRONTEND_BUILD_NODE_OPTIONS="$(FRONTEND_BUILD_NODE_OPTIONS)" -t "$(DOCKER_IMAGE_VERSION_ARM64)" -t "$(DOCKER_IMAGE_LATEST_ARM64)" --load .

publish:
	@echo "🚀 Pushing arch Docker image tags..."
	@$(DOCKER) push "$(DOCKER_IMAGE_VERSION_AMD64)"
	@$(DOCKER) push "$(DOCKER_IMAGE_VERSION_ARM64)"
	@$(DOCKER) push "$(DOCKER_IMAGE_LATEST_AMD64)"
	@$(DOCKER) push "$(DOCKER_IMAGE_LATEST_ARM64)"
	@echo "🚀 Publishing multi-arch manifests: $(DOCKER_IMAGE_VERSION) and $(DOCKER_IMAGE_LATEST)"
	@$(DOCKER_BUILDX) imagetools create -t "$(DOCKER_IMAGE_VERSION)" "$(DOCKER_IMAGE_VERSION_AMD64)" "$(DOCKER_IMAGE_VERSION_ARM64)"
	@$(DOCKER_BUILDX) imagetools create -t "$(DOCKER_IMAGE_LATEST)" "$(DOCKER_IMAGE_LATEST_AMD64)" "$(DOCKER_IMAGE_LATEST_ARM64)"

# --- WINDOWS -----------------------------------------------------

win:
	@echo "Not supported yet (windows build placeholder)"
