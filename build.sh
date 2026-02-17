#!/bin/sh
# Build script for lynxctl-rs

echo "Building lynxctl-rs for OpenBSD..."

# Build the release version
cargo build --release

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Build completed successfully!"
echo "Binary located at: ./target/release/lynxctl-rs"

# Optionally copy to a staging directory for transfer
STAGING_DIR="./staging"
mkdir -p "$STAGING_DIR"

cp "./target/release/lynxctl-rs" "$STAGING_DIR/"
cp "../README.md" "$STAGING_DIR/" 2>/dev/null || echo "README.md not found in parent directory"
cp "../LICENSE" "$STAGING_DIR/" 2>/dev/null || echo "LICENSE not found in parent directory"

echo "Files prepared in staging directory:"
ls -la "$STAGING_DIR/"

echo ""
echo "To transfer to your OpenBSD server, use:"
echo "scp -r ./staging/* user@your-server:/path/to/destination"