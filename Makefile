SHELL := /usr/bin/env bash

ROOT          := $(CURDIR)
FRONTEND_DIR  := $(ROOT)/frontend
TARGET_DIR    := $(ROOT)/target
DIST_DIR      := $(ROOT)/distributions
PACKAGING_DIR := $(ROOT)/packaging
DEB_ROOT_DIR  := $(PACKAGING_DIR)/debian
DEBIAN_DIR    := $(DEB_ROOT_DIR)/debian
DEBFILES_DIR  := $(DEB_ROOT_DIR)/deb-files
RPM_DIR       := $(PACKAGING_DIR)/rpm
RPM_SPECS_DIR := $(RPM_DIR)/SPECS
RPM_SOURCES_DIR := $(RPM_DIR)/SOURCES
NVM_DIR       ?= $(HOME)/.nvm
NODE_VERSION  ?= 24
VERSION       ?= $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH          ?= $(shell uname -m)
DOCKER        ?= docker
CARGO         ?= cargo

FRONTEND_STAMP := $(FRONTEND_DIR)/.frontend-built

.PHONY: all help clean test \
        deb deb-docker-amd64 deb-docker-arm64 \
        rpm rpm-docker-amd64 rpm-docker-arm64 rpm-package \
        mac win

all: deb mac win

help:
	@echo "Targets:"
	@echo "  make           -> build deb (amd64+arm64 in Docker), mac placeholder, win placeholder"
	@echo "  make deb       -> build Debian packages (Docker, amd64+arm64)"
	@echo "  make rpm       -> build RPM packages (Docker, amd64+arm64)"
	@echo "  make mac       -> placeholder (prints not supported)"
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
		npm run generate:proto; \
		npm run build; \
	'
	@touch "$@"

frontend: $(FRONTEND_STAMP)

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

# --- DEB (Debian/Ubuntu) ----------------------------------------------------------

deb: deb-docker-amd64 deb-docker-arm64

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
	  cd "$$ROOT"; \
	  DEB_BUILD_OPTIONS="nostrip nocheck" debuild -b -us -uc; \
	  rm -rf ../*.build ../*.buildinfo ../*.changes ../*.ddeb ../*dbgsym*.deb; \
	  mv ../lirays-scada_*[0-9].deb "$$DIST"; \
	'

deb-docker-amd64: prepare-dist frontend
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
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release; \
		VERSION=$(VERSION) CARGO=$$CARGO make deb-package; \
		rm -f ./debian ./deb-files'

deb-docker-arm64: prepare-dist frontend
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
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release; \
		VERSION=$(VERSION) CARGO=$$CARGO make deb-package; \
		rm -f ./debian ./deb-files'

# --- RPM (RHEL/Rocky/Fedora) -----------------------------------------------------

rpm: rpm-docker-amd64 rpm-docker-arm64

rpm-package: prepare-dist
	@echo "📦 Creating RPM package..."
	@bash -lc 'set -euo pipefail; \
	  ROOT="$(ROOT)"; \
	  DIST="$(DIST_DIR)"; \
	  RPM_TOP="$(RPM_DIR)"; \
	  SOURCES="$(RPM_SOURCES_DIR)"; \
	  mkdir -p "$$RPM_TOP/BUILD" "$$RPM_TOP/BUILDROOT" "$$RPM_TOP/RPMS" "$$RPM_TOP/SOURCES" "$$RPM_TOP/SPECS" "$$RPM_TOP/SRPMS"; \
	  install -Dm755 "$(TARGET_DIR)/release/lirays-scada" "$$SOURCES/lirays-scada"; \
	  rpmbuild -bb "$$RPM_TOP/SPECS/lirays-scada.spec" \
	    --define "_topdir $$RPM_TOP" \
	    --define "version $(VERSION)"; \
	  find "$$RPM_TOP/RPMS" -maxdepth 2 -name "*.rpm" -print -exec mv {} "$$DIST" \; \
	  rm -f "$$SOURCES/lirays-scada"; \
	'

rpm-docker-amd64: prepare-dist frontend
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
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release; \
		VERSION=$(VERSION) CARGO=$$CARGO make rpm-package; \
	'

rpm-docker-arm64: prepare-dist frontend
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
		VERSION=$(VERSION) CARGO=$$CARGO $$CARGO build --release; \
		VERSION=$(VERSION) CARGO=$$CARGO make rpm-package; \
	'

# --- MAC OS -----------------------------------------------------

mac:
	@echo "Not supported yet (mac build placeholder)"

# --- WINDOWS -----------------------------------------------------

win:
	@echo "Not supported yet (windows build placeholder)"
