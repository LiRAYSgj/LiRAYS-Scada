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
MAC_DIR       := $(PACKAGING_DIR)/macos
MAC_PAYLOAD   := $(MAC_DIR)/payload
MAC_BUILD_DIR := $(MAC_DIR)/build
MAC_APP       := $(MAC_PAYLOAD)/Applications/LiRAYS Scada.app
MAC_RESOURCES := $(MAC_APP)/Contents/Resources
MAC_BIN_DIR   := $(MAC_APP)/Contents/MacOS
MAC_ICON_PNG  := $(MAC_DIR)/resources/app_icon.png
MAC_ICONSET   := $(MAC_BUILD_DIR)/app.iconset
MAC_ICNS      := $(MAC_BUILD_DIR)/app_icon.icns
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
        mac mac-build-service mac-build-gui mac-icon mac-stage mac-pkg \
        win

all: deb mac win

help:
	@echo "Targets:"
	@echo "  make           -> build deb (amd64+arm64 in Docker), mac universal pkg, win placeholder"
	@echo "  make deb       -> build Debian packages (Docker, amd64+arm64)"
	@echo "  make rpm       -> build RPM packages (Docker, amd64+arm64)"
	@echo "  make mac       -> build macOS universal PKG with GUI + launchd"
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

# --- macOS (universal pkg + GUI) ------------------------------------------

mac: mac-pkg

mac-build-service:
	@echo "==> Building universal service binary"
	@rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
	@$(CARGO) build --release --target=aarch64-apple-darwin
	@$(CARGO) build --release --target=x86_64-apple-darwin
	@mkdir -p $(MAC_BUILD_DIR)
	@lipo -create \
		$(TARGET_DIR)/aarch64-apple-darwin/release/lirays-scada \
		$(TARGET_DIR)/x86_64-apple-darwin/release/lirays-scada \
		-output $(MAC_BUILD_DIR)/lirays-scada-universal

mac-build-gui:
	@echo "==> Building universal GUI app"
	@rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
	@CARGO_TARGET_DIR=$(TARGET_DIR) $(CARGO) build --release --target=aarch64-apple-darwin --manifest-path $(MAC_DIR)/gui/Cargo.toml
	@CARGO_TARGET_DIR=$(TARGET_DIR) $(CARGO) build --release --target=x86_64-apple-darwin --manifest-path $(MAC_DIR)/gui/Cargo.toml
	@mkdir -p $(MAC_BUILD_DIR)
	@lipo -create \
		$(TARGET_DIR)/aarch64-apple-darwin/release/lirays-scada-gui \
		$(TARGET_DIR)/x86_64-apple-darwin/release/lirays-scada-gui \
		-output $(MAC_BUILD_DIR)/lirays-scada-gui-universal

mac-icon:
	@echo "==> Preparing app icon (.icns)"
	@mkdir -p $(MAC_ICONSET)
	@cp $(MAC_ICON_PNG) $(MAC_ICONSET)/icon_512x512.png
	@sips -z 256 256 $(MAC_ICONSET)/icon_512x512.png --out $(MAC_ICONSET)/icon_256x256.png >/dev/null
	@sips -z 128 128 $(MAC_ICONSET)/icon_512x512.png --out $(MAC_ICONSET)/icon_128x128.png >/dev/null
	@sips -z 64 64 $(MAC_ICONSET)/icon_512x512.png --out $(MAC_ICONSET)/icon_64x64.png >/dev/null
	@sips -z 32 32 $(MAC_ICONSET)/icon_512x512.png --out $(MAC_ICONSET)/icon_32x32.png >/dev/null
	@sips -z 16 16 $(MAC_ICONSET)/icon_512x512.png --out $(MAC_ICONSET)/icon_16x16.png >/dev/null
	@iconutil -c icns $(MAC_ICONSET) -o $(MAC_ICNS)

mac-stage: mac-build-service mac-build-gui mac-icon
	@echo "==> Staging macOS payload"
	@rm -rf $(MAC_PAYLOAD)
	@mkdir -p "$(MAC_PAYLOAD)/usr/local/bin" \
		"$(MAC_PAYLOAD)/Library/LiRAYS-Scada" \
		"$(MAC_PAYLOAD)/Library/LaunchDaemons" \
		"$(MAC_PAYLOAD)/Library/Logs/LiRAYS-Scada" \
		"$(MAC_APP)/Contents/MacOS" \
		"$(MAC_APP)/Contents/Resources"
	@install -m755 $(MAC_BUILD_DIR)/lirays-scada-universal "$(MAC_PAYLOAD)/usr/local/bin/lirays-scada"
	@install -m644 $(MAC_DIR)/resources/settings.yaml "$(MAC_PAYLOAD)/Library/LiRAYS-Scada/settings.yaml.default"
	@install -m644 $(MAC_DIR)/launchd/com.lirays.scada.plist "$(MAC_PAYLOAD)/Library/LaunchDaemons/com.lirays.scada.plist"
	@install -m644 $(MAC_DIR)/launchd/com.lirays.scada.root.plist "$(MAC_PAYLOAD)/Library/LaunchDaemons/com.lirays.scada.root.plist"
	@install -m755 $(MAC_BUILD_DIR)/lirays-scada-gui-universal "$(MAC_BIN_DIR)/lirays-scada-gui"
	@install -m644 $(MAC_ICNS) "$(MAC_RESOURCES)/app_icon.icns"
	@sed 's/@VERSION@/$(VERSION)/g' $(MAC_DIR)/resources/Info.plist > "$(MAC_APP)/Contents/Info.plist"
	@plutil -convert binary1 "$(MAC_APP)/Contents/Info.plist"
	@/usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string app_icon" "$(MAC_APP)/Contents/Info.plist" >/dev/null 2>&1 || true

mac-pkg: prepare-dist mac-stage
	@echo "==> Building PKG"
	@mkdir -p $(MAC_BUILD_DIR)
	@pkgbuild \
		--root "$(MAC_PAYLOAD)" \
		--identifier com.lirays.scada \
		--version $(VERSION) \
		--scripts $(MAC_DIR)/scripts \
		$(MAC_BUILD_DIR)/LiRAYS-Scada-component.pkg
	@cp $(MAC_BUILD_DIR)/LiRAYS-Scada-component.pkg $(DIST_DIR)/LiRAYS-Scada-macos-universal.pkg

# --- WINDOWS -----------------------------------------------------

win:
	@echo "Not supported yet (windows build placeholder)"
