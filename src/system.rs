use console::style;
use crate::utils;
use crate::APP_ROOT;

/// Prints a styled status dashboard for the login MOTD
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

    // Calculate Active Peers for the dashboard
    let peer_count = utils::run_command_output("doas wg show wg0 peers | wc -l")
        .unwrap_or_else(|| "0".to_string())
        .trim()
        .to_string();

    // Dark Mode Dashboard Output
    println!();
    println!("  {}", style("   _                 ").green());
    println!("  {}", style("  / \\_   LynxEdge    ").green());
    println!("  {}", style("  \\ / \\  Appliance   ").green());
    println!("  {}", style("   \\_\\/  Status v5.0 ").green());
    println!();
    
    println!("  {} {}", style("SYSTEM CONTEXT").bold().dim(), style("─".repeat(26)).dim());
    println!("  {:<15} {}", style("WAN ENDPOINT:").dim(), style(wan_ip).yellow());
    println!("  {:<15} {}", style("DNS SHIELD:").dim(), unbound_status);
    println!("  {:<15} {}", style("VPN TUNNEL:").dim(), wg_status);
    println!("  {:<15} {}", style("ACTIVE PEERS:").dim(), style(peer_count).cyan());
    println!("  {}", style("─".repeat(40)).dim());
    println!("  {} Run {} for live metrics.", style("NOTICE:").yellow().dim(), style("lynxctl network live").cyan());
    println!();
}

/// Rebuilds the kernel state, syncs configurations, and refreshes the firewall
pub fn sync_kernel() {
    println!("{}", style("Rebuilding LynxEdge Stack...").yellow());

    // Phase 1: Interface & Peer Injection
    utils::run_command("doas ifconfig wg0 create 2>/dev/null || true");
    utils::run_command("doas ifconfig wg0 inet 10.200.200.1 255.255.255.0 up");

    let peer_cmd = format!(
        "for f in {}/etc/wireguard/clients/*.conf; do \
         pub=$(grep '# PublicKey:' \"$f\" | awk '{{print $NF}}'); \
         ip=$(grep 'Address' \"$f\" | awk '{{print $3}}' | cut -d'/' -f1); \
         if [ -n \"$pub\" ] && [ -n \"$ip\" ]; then \
         doas wg set wg0 peer \"$pub\" allowed-ips \"$ip/32\"; \
         fi; done",
        APP_ROOT
    );
    utils::run_command(&peer_cmd);

    // Phase 2: Secure Jail Synchronization
    println!(" {} Syncing Master config to Unbound Jail...", style("→").blue());
    utils::run_command(&format!("doas cp {}/etc/unbound/* /var/unbound/etc/", APP_ROOT));
    
    // Critical Fix: Mirror SSL Certs for TLS forwarding
    utils::run_command("doas mkdir -p /var/unbound/etc/ssl");
    utils::run_command("doas cp /etc/ssl/cert.pem /var/unbound/etc/ssl/cert.pem");
    utils::run_command("doas ln -sf /etc/ssl/cert.pem /var/unbound/etc/cert.pem");
    
    // Finalize Jail Permissions & Firewall
    utils::run_command("doas chown -R _unbound:_unbound /var/unbound/etc");
    utils::run_command("doas pfctl -f /etc/pf.conf");
    
    println!("{}", style("Sync Complete. Appliance is live.").green());
}

/// Fetches the latest OISD blocklist and redeploys it to the DNS worker
pub fn update_ads() {
    println!("{}", style("-> Initiating Threat Intelligence Sync...").bold());
    let out_path = format!("{}/etc/unbound/oisd_blocklist.conf", APP_ROOT);
    
    // We use -f to ensure curl returns a non-zero exit code on HTTP errors
    // We removed the -w "%{http_code}" to avoid shell interpolation bugs
    let curl_cmd = format!(
        "doas -u lynxedge curl -f -sSL --cacert /etc/ssl/cert.pem https://big.oisd.nl/unbound -o {}", 
        out_path
    );

    // Using utils::run_command which checks for exit code 0
    if utils::run_command(&curl_cmd) {
        println!(" {} Downloaded OISD Big list successfully.", style("✓").green());
        
        // Push from appliance root to service jail
        println!(" {} Deploying to Unbound Jail...", style("→").blue());
        utils::run_command(&format!("doas cp {} /var/unbound/etc/oisd_blocklist.conf", out_path));
        
        if utils::run_command("doas rcctl restart unbound") {
            println!(" {} Appliance updated and DNS shield restarted.", style("✓").green());
        }
    } else {
        eprintln!(" {} Update failed.", style("✗").red());
        eprintln!("    {} check: 1. PF Egress | 2. Write perms on {}/etc/unbound/", style("→").dim(), APP_ROOT);
    }
}

/// Performs a multi-point security and integrity audit
pub fn run_security_audit() {
    println!("{}", style("--- LynxEdge Appliance Audit v5.0 ---").bold().cyan());
    let mut issues = 0;

    if utils::run_command_output("id lynxedge").is_some() {
        println!(" {} Identity 'lynxedge' verified.", style("✓").green());
    } else {
        println!(" {} CRITICAL: 'lynxedge' user missing.", style("✗").red());
        issues += 1;
    }

    // MD5 Content Verification for partition-safe sync
    let opt_hash = utils::run_command_output(&format!("md5 -q {}/etc/unbound/unbound.conf", APP_ROOT));
    let var_hash = utils::run_command_output("md5 -q /var/unbound/etc/unbound.conf");

    if opt_hash.is_some() && opt_hash == var_hash {
        println!(" {} Jail-Bridge synchronized (Content verified).", style("✓").green());
    } else {
        println!(" {} WARNING: Jail files out of sync. Run 'system sync' to fix.", style("!").yellow());
        issues += 1;
    }

    if utils::run_command("doas -u lynxedge curl -sI --connect-timeout 2 https://google.com > /dev/null") {
        println!(" {} Appliance egress (service user) verified.", style("✓").green());
    } else {
        eprintln!(" {} ERROR: Egress blocked for 'lynxedge'. Check PF rules.", style("✗").red());
        issues += 1;
    }

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

pub fn netinfo() {
    println!("{}", style("Appliance Network Context:").bold());
    if let Some(wan) = utils::run_command_output("curl -s ifconfig.me") {
        println!(" WAN Endpoint: {}", wan.trim());
    }
    if let Some(wg) = utils::run_command_output("ifconfig wg0 | grep 'inet '") {
        println!(" Gateway:      {}", wg.trim());
    }
}

pub fn upgrade_system() {
    println!("{}", style("Starting Full System Upgrade...").cyan());
    utils::run_command("doas pkg_add -u && doas syspatch");
}