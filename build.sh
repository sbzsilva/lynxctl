#!/bin/sh
# Build & Deploy script for lynxctl v4.2

echo "Building lynxctl for OpenBSD..."

# Build the release version
cargo build --release

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Build completed successfully!"

# 1. Create dedicated group and user
if ! getent group lynxctl >/dev/null 2>&1; then
    echo "Creating dedicated service group: lynxctl..."
    doas groupadd lynxctl
fi

if ! id "lynxctl" >/dev/null 2>&1; then
    echo "Creating dedicated service user: lynxctl..."
    # -s /sbin/nologin ensures the user cannot log in
    doas useradd -s /sbin/nologin -d /var/empty -g lynxctl -c "LynxEdge Controller" lynxctl
fi

# 2. Install binary and set restricted permissions
echo "Installing lynxctl to /usr/local/bin/ ..."
doas cp target/release/lynxctl /usr/local/bin/
doas chown lynxctl:wheel /usr/local/bin/lynxctl
doas chmod 755 /usr/local/bin/lynxctl

echo "--------------------------------------------------------"
echo "INSTALL COMPLETE: lynxctl is now a restricted service."
echo "--------------------------------------------------------"