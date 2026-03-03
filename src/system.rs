use console::style;
use crate::utils;

pub fn sync_kernel() {
    println!("{}", style("Rebuilding WireGuard Kernel State...").yellow());

    // Phase 1: Interface Configuration
    println!("{} Configuring WireGuard interface...", style("→").blue());
    if utils::run_command("doas ifconfig wg0 inet 10.200.200.1 255.255.255.0 up") {
        println!(" {} Interface UP", style("✓").green());
    } else {
        eprintln!("{} Failed to configure wg0 interface", style("[ERROR]").red());
        println!("{} Attempting to create interface...", style("→").blue());
        if utils::run_command("doas ifconfig wg0 create && doas ifconfig wg0 inet 10.200.200.1 255.255.255.0 up") {
            println!(" {} Interface Created and UP", style("✓").green());
        } else {
            eprintln!("{} Failed to create and configure wg0 interface", style("[ERROR]").red());
            return;
        }
    }

    // Phase 2: Key Management
    println!("{} Loading server private key...", style("→").blue());
    if utils::run_command("doas wg set wg0 private-key /etc/wireguard/keys/server.key") {
        println!(" {} Server Key Loaded", style("✓").green());
    } else {
        eprintln!("{} Failed to set server private key", style("[ERROR]").red());
        return;
    }

    // Phase 3: Peer Synchronization
    println!("{} Syncing peers from config files...", style("→").blue());
    
    // First, remove all existing peers
    if utils::run_command("doas wg show wg0 peers | while read -r peer; do doas wg set wg0 peer \"$peer\" remove; done") {
        println!(" {} Existing peers removed", style("✓").green());
    } else {
        eprintln!("{} Failed to remove existing peers", style("[ERROR]").red());
        return;
    }

    // Now add peers from config files
    let atomic_push_cmd = concat!(
        "for f in /etc/wireguard/clients/*.conf; do ",
        "pub=$(grep 'PublicKey' $f | tail -n 1 | awk '{print $NF}'); ",
        "ip=$(grep 'Address' $f | awk '{print $3}' | cut -d'/' -f1); ",
        "if [ -n \"$pub\" ] && [ -n \"$ip\" ]; then ",
        "doas wg set wg0 peer \"$pub\" allowed-ips \"$ip/32\"; fi; done"
    );

    if utils::run_command(atomic_push_cmd) {
        println!(" {} Peers Synced", style("✓").green());
    } else {
        eprintln!("{} Failed to sync peers from config files", style("[ERROR]").red());
        return;
    }

    sync_pf();
    println!("{}", style("Sync Complete. VPN is live.").green());
}

fn sync_pf() {
    println!("{}", style("  [Syncing PF firewall...]").dim());
    
    // Fixed: Added doas to allow the lynxctl user to reload rules
    if utils::run_command("doas pfctl -f /etc/pf.conf") {
        println!("{}", style("  [PF Rules Reloaded]").green());
    } else {
        eprintln!("{} Failed to reload PF rules", style("[ERROR]").red());
    }
}

pub fn update_ads() {
    println!(" -> Fetching latest OISD blocklist (via DNS Fallback)...");
    let url = "https://big.oisd.nl/unbound";
    let out_path = "/var/unbound/etc/oisd_blocklist.conf";

    // Use Quad9 fallback to bypass local DNS issues
    let curl_cmd = format!("curl -sL --dns-servers 9.9.9.9 {} -o {}", url, out_path);

    if utils::run_command(&curl_cmd) {
        println!(" {} Update downloaded successfully.", style("✓").green());
        sync_kernel(); // Reload rules
    } else {
        eprintln!(" {} Download failed. Check PF outbound rules.", style("✗").red());
    }
}


pub fn run_security_audit() {
    println!("{}", style("--- LynxEdge Security Audit ---").bold().cyan());
    let mut issues = 0;

    // 1. Check Service User
    if utils::run_command_output("id lynxctl").is_some() {
        println!(" {} Service user 'lynxctl' exists.", style("✓").green());
    } else {
        println!(" {} CRITICAL: 'lynxctl' user is missing.", style("✗").red());
        issues += 1;
    }

    // 2. Check Directory Permissions
    let paths = ["/etc/wireguard", "/etc/wireguard/clients"];
    for path in &paths {
        let check = format!("doas test -w {} && echo 'ok'", path);
        if utils::run_command_output(&check).is_some() {
            println!(" {} Write access to {} is verified.", style("✓").green(), path);
        } else {
            println!(" {} ERROR: No write access to {}.", style("✗").red(), path);
            issues += 1;
        }
    }

    // 3. Check for Setuid Bit
    let perms = utils::run_command_output("stat -f %Sp /usr/local/bin/lynxctl").unwrap_or_default();
    if perms.contains('s') {
        println!(" {} Binary setuid bit is active.", style("✓").green());
    } else {
        println!(" {} WARNING: setuid bit missing (run build.sh).", style("✗").yellow());
        issues += 1;
    }

    // Check if qrencode is available
    if utils::run_command("which qrencode") {
        println!(" {} qrencode is installed.", style("✓").green());
    } else {
        println!(" {} qrencode MISSING. Install with: pkg_add qrencode", style("✗").red());
        issues += 1;
    }

    // Check directory permissions for client configs
    let check_dir = "doas test -r /etc/wireguard/clients && echo 'OK'";
    if utils::run_command_output(check_dir).is_some() {
        println!(" {} Service user can read client directory.", style("✓").green());
    } else {
        println!(" {} Permission denied on /etc/wireguard/clients.", style("✗").red());
        issues += 1;
    }

    println!("\nAudit finished with {} issues found.", issues);
}

pub fn upgrade_system() {
    println!("{}", style("Starting Full System Upgrade...").cyan());
    utils::run_command("doas pkg_add -u && doas syspatch");
}

pub fn netinfo() {
    println!("{} Network Information:", style("Network Information:").bold());
    
    if let Some(wan_ip) = utils::run_command_output("curl -s ifconfig.me") {
        println!("{}", wan_ip);
    } else {
        println!("Could not fetch WAN IP");
    }
    
    if let Some(wg_addr) = utils::run_command_output("ifconfig wg0 | grep 'inet '") {
        println!("{}", wg_addr);
    } else {
        println!("Could not fetch wg0 address");
    }
}