# Deployment Guide for lynxctl

This guide explains how to deploy the Rust version of lynxctl on an OpenBSD system.

## Prerequisites

Before deploying lynxctl, ensure your OpenBSD system has the required dependencies:

```bash
# Install required packages
doas pkg_add wireguard-tools unbound curl qrencode

# Ensure Rust is available (only needed for building, not runtime)
# Rust is not required on the target system if you build elsewhere
```

## Building the Binary

### On the Target System (OpenBSD)

```bash
# Install Rust (if not already installed)
doas pkg_add rust

# Clone the repository
git clone https://github.com/sbzsilva/lynxctl.git
cd lynxctl

# Build the release binary
cargo build --release

# The binary will be available at ./target/release/lynxctl
```

### Cross-compilation (from another system)

If building from a different system than the target:

```bash
# Add the OpenBSD target
rustup target add x86_64-unknown-openbsd

# Build for OpenBSD
cargo build --target x86_64-unknown-openbsd --release
```

## Installing lynxctl

After building or transferring the binary to your OpenBSD system:

```bash
# Copy the binary to system location
doas cp ./target/release/lynxctl /usr/local/bin/

# Set the correct ownership and permissions (SetUID root)
doas chown root:wheel /usr/local/bin/lynxctl
doas chmod 4755 /usr/local/bin/lynxctl

# Verify installation
lynxctl
```

## Verification

After installation, verify the binary works correctly:

```bash
# Test basic functionality
lynxctl

# Check if SetUID bit is properly set
ls -l /usr/local/bin/lynxctl
# Output should show: -rwsr-xr-x (notice the 's' in the owner permissions)
```

## Configuration

Ensure your system has the required WireGuard and Unbound configurations:

- WireGuard interface `wg0` configured
- Client configurations in `/etc/wireguard/clients/`
- Server key at `/etc/wireguard/keys/server.key`
- Unbound properly configured

## Security Notes

- The binary is designed to run with elevated privileges (SetUID root)
- Only trusted users should have execute permissions
- Regular security audits of the code are recommended
- Monitor logs for unauthorized access

## Updating

To update to a new version:

```bash
# Stop any running instances
# Build new version following above instructions

# Backup current version
doas cp /usr/local/bin/lynxctl /usr/local/bin/lynxctl.backup

# Install new version
doas cp ./target/release/lynxctl /usr/local/bin/
doas chown root:wheel /usr/local/bin/lynxctl
doas chmod 4755 /usr/local/bin/lynxctl
```

## Troubleshooting

- If commands fail with permission errors, verify SetUID permissions are set
- If system commands aren't found, ensure PATH includes `/usr/local/bin`
- Check that required services (WireGuard, Unbound) are running before using lynxctl