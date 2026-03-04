#!/bin/sh
# LynxEdge Appliance Build & Deploy v5.0

set -e

echo "--- 1. Compiling Release Binary ---"
cargo build --release

echo "--- 2. Deploying to Appliance Root ---"
doas cp target/release/lynxctl /opt/lynxedge/bin/lynxctl

echo "--- 3. Hardening Binary ---"
# Set ownership to the dedicated service identity
doas chown lynxedge:wheel /opt/lynxedge/bin/lynxctl
# Apply setuid bit so management logic runs with lynxedge privileges
doas chmod 4755 /opt/lynxedge/bin/lynxctl

echo "--- 4. Updating System Path ---"
# Create a global symlink for clean CLI access
doas ln -sf /opt/lynxedge/bin/lynxctl /usr/local/bin/lynxctl

echo "Deployment Complete. LynxEdge v5.0 is now Active."