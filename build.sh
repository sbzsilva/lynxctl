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

# 1. Create the dedicated service user if it doesn't exist
if ! id "lynxctl" >/dev/null 2>&1; then
    echo "Creating dedicated service user: lynxctl..."
    # -s /sbin/nologin prevents the user from logging in manually
    # -d /var/empty is a secure, standard practice for daemon users
    doas useradd -s /sbin/nologin -d /var/empty -g =node -c "LynxEdge Controller" lynxctl
fi

# 2. Prepare staging
STAGING_DIR="./staging"
mkdir -p "$STAGING_DIR"
cp "./target/release/lynxctl" "$STAGING_DIR/"
cp "README.md" "$STAGING_DIR/" 2>/dev/null || true
cp "LICENSE" "$STAGING_DIR/" 2>/dev/null || true

# 3. Install the binary to /usr/local/bin/
echo "Installing lynxctl to /usr/local/bin/ ..."
doas cp target/release/lynxctl /usr/local/bin/

# 4. Set Correct Ownership & Permissions
# We remove setuid (4755) and use standard permissions (755)
# The binary is owned by our new service user
doas chown lynxctl:wheel /usr/local/bin/lynxctl
doas chmod 755 /usr/local/bin/lynxctl

echo "--------------------------------------------------------"
echo "INSTALL COMPLETE: lynxctl is now a restricted service."
echo "CRITICAL: Update your /etc/doas.conf to allow the user."
echo "Example: permit nopass lynxctl as root cmd /usr/bin/wg"
echo "--------------------------------------------------------"