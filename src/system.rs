use console::style;
use crate::utils;
use crate::APP_ROOT;

pub fn sync_kernel() {
    println!("{}", style("Rebuilding LynxEdge Stack...").yellow());

    // Phase 1: WireGuard Interface
    if !utils::run_command("doas ifconfig wg0 inet 10.200.200.1 255.255.255.0 up 2>/dev/null") {
        utils::run_command("doas ifconfig wg0 create && doas ifconfig wg0 inet 10.200.200.1 255.255.255.0 up");
    }

    // Phase 2: Key Logic
    let key_cmd = format!("doas wg set wg0 private-key {}/etc/wireguard/keys/server.key", APP_ROOT);
    utils::run_command(&key_cmd);

    // Phase 3: Peer Sync from Appliance Path
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

    // Phase 4: Firewall & Jail Sync
    utils::run_command(&format!("doas cp {}/etc/unbound/* /var/unbound/etc/", APP_ROOT));
    utils::run_command("doas pfctl -f /etc/pf.conf");
    println!("{}", style("Sync Complete. Appliance is live.").green());
}

pub fn update_ads() {
    println!("{}", style("-> Initiating Threat Intelligence Sync...").bold());
    let out_path = format!("{}/etc/unbound/oisd_blocklist.conf", APP_ROOT);
    
    // Attempt download as service user
    let curl_cmd = format!("doas -u lynxedge curl -sL --dns-servers 9.9.9.9 https://big.oisd.nl/unbound -o {}", out_path);

    if utils::run_command(&curl_cmd) {
        println!(" {} Downloaded OISD Big list.", style("✓").green());
        
        println!(" {} Deploying to Unbound Jail...", style("→").blue());
        utils::run_command(&format!("doas cp {} /var/unbound/etc/oisd_blocklist.conf", out_path));
        
        if utils::run_command("doas rcctl restart unbound") {
            println!(" {} Appliance updated and DNS shield restarted.", style("✓").green());
        }
    } else {
        eprintln!(" {} Update failed. Check appliance egress rules in PF.", style("✗").red());
    }
}

pub fn run_security_audit() {
    println!("{}", style("--- LynxEdge Appliance Audit v5.0 ---").bold().cyan());
    let mut issues = 0;

    // Check identity
    if utils::run_command_output("id lynxedge").is_some() {
        println!(" {} Identity 'lynxedge' verified.", style("✓").green());
    } else {
        println!(" {} CRITICAL: 'lynxedge' user missing.", style("✗").red());
        issues += 1;
    }

    // Jail-Bridge Check
    let opt_inode = utils::run_command_output("ls -i /opt/lynxedge/etc/unbound/unbound.conf | awk '{print $1}'");
    let var_inode = utils::run_command_output("ls -i /var/unbound/etc/unbound.conf | awk '{print $1}'");
    if opt_inode == var_inode && opt_inode.is_some() {
        println!(" {} Jail-Bridge synchronized.", style("✓").green());
    } else {
        println!(" {} WARNING: Jail files out of sync.", style("!").yellow());
        issues += 1;
    }

    // Path Write Access
    let paths = [format!("{}/etc/wireguard/keys", APP_ROOT), format!("{}/logs", APP_ROOT)];
    for path in &paths {
        if utils::run_command(&format!("doas test -w {}", path)) {
            println!(" {} Write access to {} verified.", style("✓").green(), path);
        } else {
            eprintln!(" {} ERROR: Path {} inaccessible.", style("✗").red(), path);
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
        println!(" Gateway: {}", wg.trim());
    }
}

pub fn upgrade_system() {
    println!("{}", style("Starting Full System Upgrade...").cyan());
    utils::run_command("doas pkg_add -u && doas syspatch");
}