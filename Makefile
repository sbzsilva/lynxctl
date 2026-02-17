# Makefile for lynxctl-rs
#
# Rust-based management tool for WireGuard VPN tunnels and Unbound DNS security
#

# Binary name and location
TARGET = lynxctl-rs
BIN_PATH = /usr/local/bin
BUILD_DIR = target/release

# Build the release version of the binary
.PHONY: build
build:
	cargo build --release

# Install the binary with proper permissions
.PHONY: install
install: build
	@echo "Installing $(TARGET) to $(BIN_PATH)..."
	@# Copy the binary to the system location
	doas cp $(BUILD_DIR)/$(TARGET) $(BIN_PATH)/
	@# Set proper ownership (root with wheel group)
	doas chown root:wheel $(BIN_PATH)/$(TARGET)
	@# Set SetUID root permissions (chmod 4755)
	doas chmod 4755 $(BIN_PATH)/$(TARGET)
	@echo "Installation complete."

# Uninstall the binary
.PHONY: uninstall
uninstall:
	@echo "Removing $(TARGET) from $(BIN_PATH)..."
	doas rm -f $(BIN_PATH)/$(TARGET)
	@echo "Uninstallation complete."

# Clean build artifacts
.PHONY: clean
clean:
	cargo clean

# Run tests
.PHONY: test
test:
	cargo test

# Format code
.PHONY: fmt
fmt:
	cargo fmt

# Check code for errors without building
.PHONY: check
check:
	cargo check

# Run clippy lints
.PHONY: clippy
clippy:
	cargo clippy

# Build and create a staging directory for transfer
.PHONY: stage
stage:
	cargo build --release
	mkdir -p staging
	cp $(BUILD_DIR)/$(TARGET) staging/
	@echo "Files prepared in staging directory:"
	@ls -la staging/

# Show help
.PHONY: help
help:
	@echo "LynxEdge Rust Control Interface - Makefile options:"
	@echo ""
	@echo "  build     - Build the release binary"
	@echo "  install   - Build and install with SetUID permissions"
	@echo "  uninstall - Remove the installed binary"
	@echo "  clean     - Remove build artifacts"
	@echo "  test      - Run unit tests"
	@echo "  fmt       - Format the code"
	@echo "  check     - Check code for errors"
	@echo "  clippy    - Run clippy lints"
	@echo "  stage     - Build and prepare for transfer to server"
	@echo "  help      - Show this help message"