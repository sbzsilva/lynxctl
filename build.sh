#!/bin/sh
# Build & Deploy script for lynxctl v4.2

echo "Building lynxctl for OpenBSD..."
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
    doas useradd -s /sbin/nologin -d /var/empty -g lynxctl -c "LynxEdge Controller" lynxctl
fi

# 2. Setup WireGuard Directory Permissions
# The service user MUST own these to create profiles without 'Operation not permitted'
echo "Configuring WireGuard directory ownership..."
doas mkdir -p /etc/wireguard/clients
doas chown -R lynxctl:wheel /etc/wireguard
doas chmod 750 /etc/wireguard
doas chmod 700 /etc/wireguard/clients

# 3. Install binary and set SETUID bit
echo "Installing lynxctl to /usr/local/bin/ ..."
doas cp target/release/lynxctl /usr/local/bin/
doas chown lynxctl:wheel /usr/local/bin/lynxctl
# 4755 allows the binary to run as owner (lynxctl) regardless of who launches it
doas chmod 4755 /usr/local/bin/lynxctl

echo "--------------------------------------------------------"
echo "INSTALL COMPLETE: You can now run 'lynxctl' directly."
echo "--------------------------------------------------------"