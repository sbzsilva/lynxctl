use std::fs;
use console::style;

use crate::utils;

pub fn sync_kernel() {
    println!("{}", style("Rebuilding WireGuard Kernel State...").yellow());

    // 1. Re-initialize interface address
    if !utils::run_command("ifconfig wg0 inet 10.200.200.1 255.255.255.0 up") {
        eprintln!("{} Failed to configure wg0 interface", style("[ERROR]").red());
        return;
    }

    // 2. Load Server Private Key
    if !utils::run_command("wg set wg0 private-key /etc/wireguard/keys/server.key") {
        eprintln!("{} Failed to set server private key", style("[ERROR]").red());
        return;
    }

    // 3. Purge existing peers from kernel to ensure clean sync
    if !utils::run_command(
        "wg show wg0 peers | while read -r peer; do wg set wg0 peer $peer remove; done"
    ) {
        eprintln!("{} Failed to purge existing peers", style("[ERROR]").red());
        return;
    }

    // 4. Atomic push from .conf files to Kernel
    let atomic_push_cmd = concat!(
        "for f in /etc/wireguard/clients/*.conf; do ",
        "pub=$(grep 'PublicKey =' $f | awk '{print $3}'); ",
        "ip=$(grep 'Address =' $f | awk '{print $3}' | cut -d/ -f1); ",
        "if [ -n \"$pub\" ] && [ -n \"$ip\" ]; then ",
        "doas wg set wg0 peer \"$pub\" allowed-ips \"$ip/32\"; fi; done"
    );

    if !utils::run_command(atomic_push_cmd) {
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

    println!(" -> Fetching latest OISD blocklist...");
    let url = "https://big.oisd.nl/unbound";
    let out_path = "/var/unbound/etc/oisd_blocklist.conf";

    if !utils::run_command(&format!("curl -sL {} -o {}", url, out_path)) {
        eprintln!("{} Download failed.", style("[ERROR]").red());
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