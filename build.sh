#!/bin/sh
# Build script for lynxctl

echo "Building lynxctl for OpenBSD..."

# Build the release version
cargo build --release

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Build completed successfully!"
echo "Binary located at: ./target/release/lynxctl"

# Optionally copy to a staging directory for transfer
STAGING_DIR="./staging"
mkdir -p "$STAGING_DIR"

cp "./target/release/lynxctl" "$STAGING_DIR/"
cp "README.md" "$STAGING_DIR/" 2>/dev/null || echo "README.md not found in current directory"
cp "LICENSE" "$STAGING_DIR/" 2>/dev/null || echo "LICENSE not found in current directory"

echo "Files prepared in staging directory:"
ls -la "$STAGING_DIR/"

# Install the binary to /usr/local/bin/
echo ""
echo "Installing lynxctl to /usr/local/bin/ ..."
doas cp target/release/lynxctl /usr/local/bin/

# Set correct ownership
doas chown root:wheel /usr/local/bin/lynxctl

# Set correct permissions (setuid with 4755)
doas chmod 4755 /usr/local/bin/lynxctl

echo "lynxctl installed successfully to /usr/local/bin/lynxctl"