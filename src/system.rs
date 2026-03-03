use std::fs;
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
    
    if utils::run_command("pfctl -f /etc/pf.conf") {
        println!("{}", style("  [PF Rules Reloaded]").green());
    } else {
        eprintln!("{} Failed to reload PF rules", style("[ERROR]").red());
    }
}

pub fn update_ads() {
    println!("{}", style("Starting Integrated OISD Update...").yellow());
    
    fix_unbound_permissions();

    println!(" -> Fetching latest OISD blocklist (via DNS Fallback)...");
    let url = "https://big.oisd.nl/unbound";
    let out_path = "/var/unbound/etc/oisd_blocklist.conf";

    // Use Quad9 (9.9.9.9) as a fallback resolver so curl works if Unbound is down
    let curl_cmd = format!("curl -sL --dns-servers 9.9.9.9 {} -o {}", url, out_path);

    if !utils::run_command(&curl_cmd) {
        eprintln!("{} Download failed. Check network connectivity.", style("[ERROR]").red());
        return;
    }

    println!(" -> Validating Unbound configuration...");
    if utils::run_command("unbound-checkconf -q > /dev/null") {
        if utils::run_command("rcctl restart unbound") {
            if utils::is_service_running("unbound") {
                println!("{} DNS Shield updated and active.", 
                    style("[SUCCESS]").green());
            } else {
                eprintln!("{} Unbound failed to restart.", 
                    style("[ERROR]").red());
            }
        } else {
            eprintln!("{} Failed to restart unbound.", 
                style("[ERROR]").red());
        }
    } else {
        eprintln!("{} Syntax validation failed. Reverting.", 
            style("[ERROR]").red());
        let _ = fs::remove_file(out_path);
    }
}

fn fix_unbound_permissions() {
    println!("{}", style("  [Applying OpenBSD Chroot Security Fixes...]").dim());
    utils::run_command("chown root:_unbound /var/unbound/etc");
    utils::run_command("chmod 775 /var/unbound/etc");
    utils::run_command("chown _unbound:_unbound /var/unbound/db/root.key >/dev/null 2>&1");
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