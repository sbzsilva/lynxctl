use console::style;
use crate::utils;
use crate::APP_ROOT;

/// Prints a status dashboard to the console on the console
pub fn print_motd_status() {
    let wan_ip = utils::run_command_output("curl -s ifconfig.me").unwrap_or_else(|| "OFFLINE".to_string());
    
    // Service Status Icons
    let unbound_status = if utils::is_service_running("unbound") {
        style("● ACTIVE").green().bold()
    } else {
        style("○ FAILED").red().blink()
    };

    let wg_status = if utils::run_command_output("ifconfig wg0").is_some() {
        style("● ACTIVE").green().bold()
    } else {
        style("○ DOWN").red()
    };

    // Dark Mode Dashboard Output
    println!();
    println!("  {}", style("   _                 ").green());
    println!("  {}", style("  / \\_   LynxEdge    ").green());
    println!("  {}", style("  \\ / \\  Appliance   ").green());
    println!("  {}", style("   \\_\\/  Status v5.0 ").green());
    println!();
    println!("  {} {}", style("SYSTEM CONTEXT").bold().dim(), "─".repeat(26));
    println!("  {:<15} {}", style("WAN ENDPOINT:").dim(), style(wan_ip).yellow());
    println!("  {:<15} {}", style("DNS SHIELD:").dim(), unbound_status);
    println!("  {:<15} {}", style("VPN TUNNEL:").dim(), wg_status);
    println!("  {}", "─".repeat(40).dim());
    println!("  {} Run {} for live metrics.", style("NOTICE:").yellow().dim(), style("lynxctl network live").cyan());
    println!();
}

/// Rebuilds the kernel state, syncs configurations, and refreshes the firewall
pub fn sync_kernel() {
    println!("{}", style("Rebuilding LynxEdge Stack...").yellow());

    // Phase 1: WireGuard Interface Management
    // Ensures wg0 is active with the correct internal gateway IP
    if !utils::run_command("doas ifconfig wg0 inet 10.200.200.1 255.255.255.0 up 2>/dev/null") {
        utils::run_command("doas ifconfig wg0 create && doas ifconfig wg0 inet 10.200.200.1 255.255.255.0 up");
    }

    // Phase 2: Server Key Application
    let key_cmd = format!("doas wg set wg0 private-key {}/etc/wireguard/keys/server.key", APP_ROOT);
    utils::run_command(&key_cmd);

    // Phase 3: Peer Synchronization
    // Flushes current peers and re-injects them from the appliance profile directory
    utils::run_command("doas wg show wg0 peers | while read -r peer; do doas wg set wg0 peer \"$peer\" remove; done");

    let atomic_push_cmd = format!(
        "for f in {}/etc/wireguard/clients/*.conf; do \
        pub=$(grep 'PublicKey' $f | tail -n 1 | awk '{{print $NF}}'); \
        ip=$(grep 'Address' $f | awk '{{print $3}}' | cut -d'/' -f1); \
        if [ -n \"$pub\" ] && [ -n \"$ip\" ]; then \
        doas wg set wg0 peer \"$pub\" allowed-ips \"$ip/32\"; fi; done",
        APP_ROOT
    );
    utils::run_command(&atomic_push_cmd);

    // Phase 4: Partition-Safe Jail Synchronization
    // Uses atomic copy to bridge configurations across disk partitions (e.g., /opt to /var)
    println!(" {} Syncing Master config to Unbound Jail...", style("→").blue());
    utils::run_command(&format!("doas cp {}/etc/unbound/* /var/unbound/etc/", APP_ROOT));
    utils::run_command("doas chown -R root:wheel /var/unbound/etc");
    
    // Refresh Packet Filter rules
    utils::run_command("doas pfctl -f /etc/pf.conf");
    println!("{}", style("Sync Complete. Appliance is live.").green());
}

/// Fetches the latest OISD blocklist and redeploys it to the DNS worker
pub fn update_ads() {
    println!("{}", style("-> Initiating Threat Intelligence Sync...").bold());
    let out_path = format!("{}/etc/unbound/oisd_blocklist.conf", APP_ROOT);
    
    // Attempt download as the dedicated service user to verify egress permissions
    let curl_cmd = format!(
        "doas -u lynxedge curl -sL --dns-servers 9.9.9.9 https://big.oisd.nl/unbound -o {}", 
        out_path
    );

    if utils::run_command(&curl_cmd) {
        println!(" {} Downloaded OISD Big list.", style("✓").green());
        
        println!(" {} Deploying to Unbound Jail...", style("→").blue());
        utils::run_command(&format!("doas cp {} /var/unbound/etc/oisd_blocklist.conf", out_path));
        
        if utils::run_command("doas rcctl restart unbound") {
            println!(" {} Appliance updated and DNS shield restarted.", style("✓").green());
        }
    } else {
        eprintln!(
            " {} Update failed. Verify that user 'lynxedge' has egress permission in PF.", 
            style("✗").red()
        );
    }
}

/// Performs a multi-point security and integrity audit of the appliance
pub fn run_security_audit() {
    println!("{}", style("--- LynxEdge Appliance Audit v5.0 ---").bold().cyan());
    let mut issues = 0;

    // 1. Identity Verification
    if utils::run_command_output("id lynxedge").is_some() {
        println!(" {} Identity 'lynxedge' verified.", style("✓").green());
    } else {
        println!(" {} CRITICAL: 'lynxedge' user missing.", style("✗").red());
        issues += 1;
    }

    // 2. Jail-Bridge Integrity (MD5 Content Verification)
    // Bypasses Inode limitations on split partitions by comparing file hashes
    let opt_hash = utils::run_command_output(&format!("md5 -q {}/etc/unbound/unbound.conf", APP_ROOT));
    let var_hash = utils::run_command_output("md5 -q /var/unbound/etc/unbound.conf");

    if opt_hash.is_some() && opt_hash == var_hash {
        println!(" {} Jail-Bridge synchronized (Content verified).", style("✓").green());
    } else {
        println!(" {} WARNING: Jail files out of sync. Run 'system sync' to fix.", style("!").yellow());
        issues += 1;
    }

    // 3. Service Egress Validation
    // Tests if the 'lynxedge' identity can actually reach the internet via PF
    if utils::run_command("doas -u lynxedge curl -sI --connect-timeout 2 https://google.com > /dev/null") {
        println!(" {} Appliance egress (service user) verified.", style("✓").green());
    } else {
        eprintln!(" {} ERROR: Egress blocked for 'lynxedge'. Check PF rules.", style("✗").red());
        issues += 1;
    }

    // 4. Filesystem Write Permissions
    let paths = [
        format!("{}/etc/wireguard/keys", APP_ROOT), 
        format!("{}/logs", APP_ROOT)
    ];
    for path in &paths {
        if utils::run_command(&format!("doas test -w {}", path)) {
            println!(" {} Write access to {} verified.", style("✓").green(), path);
        } else {
            eprintln!(" {} ERROR: Path {} inaccessible or read-only.", style("✗").red(), path);
            issues += 1;
        }
    }

    println!("\nAudit finished with {} issues.", issues);
}

/// Displays external and internal network context
pub fn netinfo() {
    println!("{}", style("Appliance Network Context:").bold());
    if let Some(wan) = utils::run_command_output("curl -s ifconfig.me") {
        println!(" WAN Endpoint: {}", wan.trim());
    }
    if let Some(wg) = utils::run_command_output("ifconfig wg0 | grep 'inet '") {
        println!(" Gateway:      {}", wg.trim());
    }
}

/// Standard OpenBSD system maintenance
pub fn upgrade_system() {
    println!("{}", style("Starting Full System Upgrade...").cyan());
    utils::run_command("doas pkg_add -u && doas syspatch");
}