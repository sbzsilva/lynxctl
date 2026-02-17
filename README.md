# LynxEdge Control Interface (lynxctl)

A high-performance, consolidated Rust management suite for secure network gateways on OpenBSD, ported from the original C implementation.

## Overview

lynxctl is a unified management tool designed to oversee WireGuard VPN tunnels and Unbound DNS security. It maintains all the functionality of the original C implementation but with improved safety and modern language features provided by Rust.

## Key Features

- **Memory Safety**: Benefit from Rust's ownership model to prevent buffer overflows and other memory-related vulnerabilities.
- **Consolidated Core**: Single binary architecture with no external shell script dependencies.
- **Atomic Kernel Sync**: Synchronizes WireGuard peers from the filesystem directly into the OpenBSD kernel.
- **Integrated Ad-Blacking**: Built-in engine to update OISD blocklists and validate Unbound configurations.
- **Smart Peer Mapping**: Automatically resolves WireGuard public keys to human-readable profile names.
- **Live Intelligence Dashboard**: Real-time tracking of VPN load, DNS cache hits, and peer data usage (scaled to KB/MB/GB).

## Requirements

- OpenBSD 7.x (AMD64/ARM64)
- Rust 1.70+ (with Cargo) - only needed for building
- WireGuard and Unbound installed
- curl and qrencode for updates and QR generation

## Building

```bash
# Clone the repository
git clone https://github.com/sbzsilva/lynxctl.git
cd lynxctl

# Build the binary using the Makefile
make build

# Or directly with Cargo
cargo build --release
```

## Installation

### Prerequisites
- OpenBSD 7.x (AMD64/ARM64)
- unbound and wireguard-tools installed.
- curl and qrencode for list updates and QR generation.

### Using the Makefile (Recommended)
```bash
# Build and install with proper SetUID permissions
make install
```

### Manual Installation
```bash
# Build the binary
cargo build --release

# Copy to system location
sudo cp target/release/lynxctl /usr/local/bin/

# Set proper permissions (Sets SetUID root permissions)
sudo chown root:wheel /usr/local/bin/lynxctl
sudo chmod 4755 /usr/local/bin/lynxctl
```

## Usage

### System Commands
- `lynxctl status`: Display a high-level health check of all services.
- `lynxctl live`: Launch the real-time Intelligence Dashboard.
- `lynxctl sync`: Reconcile filesystem configurations with the live kernel state.
- `lynxctl update`: Download and apply the latest OISD DNS blocklists.

### User Management
- `lynxctl users list`: Show all registered profiles and their assigned IPs.
- `lynxctl users create [name]`: Generate a new peer, stamp metadata, and show QR code.
- `lynxctl users qr [name]`: Re-display the QR code for an existing profile.
- `lynxctl users delete [name]`: Remove a peer from the system and kernel.

## Project Structure

- `src/main.rs`: CLI entry point and command routing.
- `src/users.rs`: User management functions.
- `src/network.rs`: Network operations and dashboard.
- `src/system.rs`: System-level operations.
- `src/monitor.rs`: Monitoring and dashboard functionality.
- `src/utils.rs`: Shared utility functions.
- `Cargo.toml`: Dependencies and build configuration.
- `Makefile`: Build and installation automation.

## Security & Permissions

lynxctl utilizes the SetUID root bit (chmod 4755) to allow authorized users to manage network interfaces and system daemons without requiring a password for every sub-command. On OpenBSD, it is designed to respect the _unbound chroot environment.

## License

Distributed under the MIT License. See LICENSE for more information.