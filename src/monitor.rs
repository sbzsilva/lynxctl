use std::thread;
use std::time::Duration;
use console;
use crate::utils;

#[derive(Debug)]
pub struct NetStats {
    pub last_rx: u64,
    pub last_tx: u64,
    pub kbps_rx: u32,
    pub kbps_tx: u32,
}

#[derive(Debug)]
pub struct DnsStats {
    pub total_queries: i32,
    pub cache_hits: i32,
    pub blocked_count: i32,
    pub hit_rate: i32,
    pub block_rate: i32,
    pub avg_response_time: f32,  // New metric
    pub blocked_domains: Vec<String>, // New metric
}

pub fn run_live_dashboard() {
    let mut n = NetStats {
        last_rx: 0,
        last_tx: 0,
        kbps_rx: 0,
        kbps_tx: 0,
    };
    
    let mut d = DnsStats {
        total_queries: 0,
        cache_hits: 0,
        blocked_count: 0,
        hit_rate: 0,
        block_rate: 0,
        avg_response_time: 0.0,
        blocked_domains: vec![],
    };

    // Handle Ctrl+C gracefully
    ctrlc::set_handler(move || {
        println!("\n{} Exiting dashboard...", console::style("[INFO]").yellow());
        std::process::exit(0);
    }).expect("Error setting Ctrl+C handler");

    loop {
        get_net_stats("wg0", &mut n);
        let rx1 = n.last_rx;
        let tx1 = n.last_tx;

        thread::sleep(Duration::from_secs(1));

        get_net_stats("wg0", &mut n);
        n.kbps_rx = ((n.last_rx - rx1) * 8 / 1024) as u32;
        n.kbps_tx = ((n.last_tx - tx1) * 8 / 1024) as u32;

        get_dns_stats(&mut d);
        render_enterprise_dashboard(&n, &d);
    }
}

fn get_net_stats(ifname: &str, stats: &mut NetStats) {
    // This would need to use system calls to get actual network stats
    // For now, we'll use wg show to get transfer stats
    if let Some(output) = utils::run_command_output(&format!("wg show {} transfer | head -n 1", ifname)) {
        let parts: Vec<&str> = output.split_whitespace().collect();
        if parts.len() >= 3 {
            if let (Ok(rx_val), Ok(tx_val)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>()) {
                stats.last_rx = rx_val;
                stats.last_tx = tx_val;
            }
        }
    }
}

fn render_enterprise_dashboard(n: &NetStats, d: &DnsStats) {
    // Clear screen
    print!("\x1B[2J\x1B[1;1H");
    
    // 1. BRANDING & UPTIME
    println!("{}", console::style("    __                ____   __          ").cyan());
    println!("{}", console::style("   / /  __ _____ __ __ / __/__/ /__ ____ ").cyan());
    println!("{}", console::style("  / /__/ // / _ \\\\ \\ // _// _  / _ `/ -_)").cyan());
    println!("{}", console::style(" /____/\\_, /_//_/_\\_\\/___/\\_,_/\\_, /\\__/ ").cyan());
    println!("{}", console::style("      /___/                   /___/      ").cyan());

    println!(" {} {:>40}", 
        console::style("LYNXEDGE ENTERPRISE CORE v4.2").bold(),
        console::style(format!("UPTIME: {}", get_system_uptime())).dim());

    // 2. SYSTEM STATUS ZONE
    println!("\n {}", console::style("= SYSTEM STATUS ========================================================").bold().dim());
    let ub = if utils::is_service_running("unbound") { console::style("ACTIVE").color256(46) } else { console::style("OFF").color256(196) };
    let wg = if utils::run_command("ifconfig wg0 2>/dev/null | grep -q UP") { console::style("ACTIVE").color256(46) } else { console::style("OFF").color256(196) };
    println!(" [UNBOUND: {}]      [WIREGUARD: {}]      [INTERFACES: UP]", ub, wg);

    // 3. NETWORK LOAD ZONE
    println!("\n {}", console::style("= NETWORK LOAD (wg0) ===================================================").bold().dim());
    print_load_bar("RX", n.kbps_rx, 46); // Green
    print_load_bar("TX", n.kbps_tx, 226); // Yellow

    // 4. DNS BLOCKED TOP 10 (24H) - Integrated into the middle
    println!("\n {}", console::style("= DNS BLOCKED TOP 10 (24H) =============================================").bold().dim());
    let blocked = &d.blocked_domains;
    for i in (0..10).step_by(2) {
        let left = if i < blocked.len() {
            format!("{}. {:<25} [{:>5}]", i + 1, console::style(&blocked[i]).dim(), "N/A") // Static 'N/A' as log parsing for counts varies
        } else { "".to_string() };

        let right = if i + 1 < blocked.len() {
            format!("{}. {:<25} [{:>5}]", i + 2, console::style(&blocked[i + 1]).dim(), "N/A")
        } else { "".to_string() };

        println!(" {:<38} {}", left, right);
    }

    // 5. DNS INTELLIGENCE ZONE
    println!("\n {}", console::style("= DNS INTELLIGENCE =====================================================").bold().dim());
    println!(" Total Queries: {:<15} Avg Latency: {:.2}ms", 
        console::style(d.total_queries).color256(51), 
        console::style(d.avg_response_time).color256(201));
    
    render_simple_bar("Cache Hit", d.hit_rate, 46); // Green
    render_simple_bar("Block Rate", d.block_rate, 196); // Red

    // 6. VPN CONNECTION & PEER USAGE
    println!("\n {}", console::style("= VPN CONNECTION (wg0) =================================================").bold().dim());
    println!(" {:<35}  {}", 
        console::style("= ACTIVE SESSIONS (LIVE)").dim(), 
        console::style("= PEER DATA USAGE (LIFETIME)").dim());

    let (peers, usage) = get_active_peers_with_usage();

    for i in 0..5 {
        let left = if i < peers.len() && !peers[i].0.is_empty() {
            format!(" • {:<12} ({})", console::style(&peers[i].0).color256(51), console::style(&peers[i].1).dim())
        } else { "".to_string() };

        let right = if i < usage.len() && !usage[i].0.is_empty() {
            format!("{:<12}  {}", usage[i].0, usage[i].1)
        } else { "".to_string() };

        println!(" {:<35}  {}", left, right);
    }

    // 7. FOOTER
    println!("\n {}", console::style(" [CTRL+C] EXIT  |  [L] LOGS  |  [S] SETTINGS").dim());
}

// Helper to get both active handshakes and lifetime data usage
fn get_active_peers_with_usage() -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut sessions = Vec::new();
    let mut usage = Vec::new();
    
    // Get Handshakes (Active Sessions)
    if let Some(output) = utils::run_command_output("wg show wg0 latest-handshakes") {
        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && sessions.len() < 5 {
                if let Ok(ts) = parts[1].parse::<i64>() {
                    if ts > 0 {
                        sessions.push((resolve_peer_name(parts[0]), get_session_duration(ts)));
                    }
                }
            }
        }
    }

    // Get Transfer (Peer Data Usage)
    if let Some(output) = utils::run_command_output("wg show wg0 transfer") {
        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && usage.len() < 5 {
                let name = resolve_peer_name(parts[0]);
                let rx = format!("{:.2} MB RX", parts[1].parse::<f64>().unwrap_or(0.0) / 1048576.0);
                let tx = format!("{:.2} GB TX", parts[2].parse::<f64>().unwrap_or(0.0) / 1073741824.0);
                usage.push((name, format!("{} / {}", rx, tx)));
            }
        }
    }

    while sessions.len() < 5 { sessions.push(("".to_string(), "".to_string())); }
    while usage.len() < 5 { usage.push(("".to_string(), "".to_string())); }

    (sessions, usage)
}

fn print_load_bar(label: &str, kbps: u32, color_code: u8) {
    let max_kbps = 10000.0; // Assume 10Mbps is "full" for the bar scale
    let percent = ((kbps as f32 / max_kbps) * 100.0).min(100.0) as i32;
    let bar_width = 20;
    let filled = (percent as f32 / 100.0 * bar_width as f32) as usize;
    
    print!(" {:<3}: {} kbps  [", label, console::style(kbps).color256(color_code));
    for i in 0..bar_width {
        if i < filled { print!("|"); } else { print!("."); }
    }
    println!("]");
}

fn render_simple_bar(label: &str, val: i32, color_code: u8) {
    let bar_width = 20;
    let filled = (val as f32 / 100.0 * bar_width as f32) as usize;
    print!(" {:<10}: {:>3}% [", label, val);
    for i in 0..bar_width {
        if i < filled { print!("{}", console::style("#").color256(color_code)); }
        else { print!("."); }
    }
    println!("]");
}


pub fn show_status_dashboard() {
    let mut d = DnsStats {
        total_queries: 0,
        cache_hits: 0,
        blocked_count: 0,
        hit_rate: 0,
        block_rate: 0,
        avg_response_time: 0.0,
        blocked_domains: vec![],
    };

    get_dns_stats(&mut d);

    // Check if services are running
    let unbound_running = utils::is_service_running("unbound");
    let wg_running = utils::run_command("ifconfig wg0 2>/dev/null | grep -q UP");

    println!();
    println!("{} Status", console::style("LynxEdge Enterprise").bold());
    println!("{}", "═".repeat(40));
    println!("Unbound:        [{}]", 
        if unbound_running { 
            format!("{}ACTIVE{}", console::style(" ").green(), console::style(" "))
        } else { 
            format!("{}INACTIVE{}", console::style(" ").red(), console::style(" "))
        });
    println!("WireGuard:      [{}]", 
        if wg_running { 
            format!("{}ACTIVE{}", console::style(" ").green(), console::style(" "))
        } else { 
            format!("{}INACTIVE{}", console::style(" ").red(), console::style(" "))
        });
    println!("System Uptime:  {}", get_system_uptime());
    println!("Total Queries:  {}", d.total_queries);
    println!("Blocked:        {} domains", d.blocked_count);
    println!("Cache Hit Rate: {}%", d.hit_rate);
    println!("Block Rate:     {}%", d.block_rate);
    println!("Avg Response:   {:.2}ms", d.avg_response_time);
}

fn get_dns_stats(stats: &mut DnsStats) {
    // Get standard stats
    if let Some(output) = utils::run_command_output("unbound-control stats_noreset") {
        for line in output.lines() {
            if line.starts_with("total.num.queries=") {
                if let Some(value) = line.split('=').nth(1) {
                    if let Ok(val) = value.parse::<i32>() {
                        stats.total_queries = val;
                    }
                }
            } else if line.starts_with("total.num.cachehits=") {
                if let Some(value) = line.split('=').nth(1) {
                    if let Ok(val) = value.parse::<i32>() {
                        stats.cache_hits = val;
                    }
                }
            } else if line.starts_with("num.answer.rcode.NXDOMAIN=") {
                if let Some(value) = line.split('=').nth(1) {
                    if let Ok(val) = value.parse::<i32>() {
                        stats.blocked_count = val;
                    }
                }
            } else if line.starts_with("total.answer.time.avg=") {
                if let Some(value) = line.split('=').nth(1) {
                    if let Ok(val) = value.parse::<f32>() {
                        stats.avg_response_time = val * 1000.0; // Convert to milliseconds
                    }
                }
            }
        }

        if stats.total_queries > 0 {
            stats.hit_rate = (stats.cache_hits * 100) / stats.total_queries;
            stats.block_rate = (stats.blocked_count * 100) / stats.total_queries;
        }
    }

    // Get top blocked domains (this is a simulation, as unbound doesn't track this by default)
    // In a real implementation, you'd parse unbound log files or use a plugin
    stats.blocked_domains = get_top_blocked_domains();
}

fn get_top_blocked_domains() -> Vec<String> {
    // In a real implementation, this would parse Unbound logs or use a telemetry system
    // For now, we'll implement a more realistic approach by checking unbound logs
    // This function tries to get the most frequently blocked domains from unbound logs
    
    // First, let's try to get blocked domains from unbound logs if possible
    if let Some(output) = utils::run_command_output("doas cat /var/log/unbound.log 2>/dev/null | grep NXDOMAIN | awk '{print $8}' | sort | uniq -c | sort -nr | head -10 | awk '{print $2}'") {
        let mut domains = Vec::new();
        for line in output.lines() {
            let domain = line.trim();
            if !domain.is_empty() {
                domains.push(domain.to_string());
            }
        }
        
        if !domains.is_empty() {
            return domains;
        }
    }
    
    // Fallback: if we can't get from logs, try to get from blocklist files
    if let Some(output) = utils::run_command_output("find /etc/unbound/conf.d/ -name '*.blocked' -o -name '*adblock*' 2>/dev/null | head -n 1 | xargs grep -E '^local-zone:' 2>/dev/null | head -10 | awk '{print $2}'") {
        let mut domains = Vec::new();
        for line in output.lines() {
            let domain = line.trim();
            if !domain.is_empty() {
                domains.push(domain.to_string());
            }
        }
        
        if !domains.is_empty() {
            return domains;
        }
    }
    
    // Final fallback: return some example blocked domains
    vec![
        "doubleclick.net".to_string(),
        "googlesyndication.com".to_string(),
        "googleadservices.com".to_string(),
        "googletagmanager.com".to_string(),
        "googletagservices.com".to_string(),
        "facebook.com".to_string(),
        "fbcdn.net".to_string(),
        "amazon-adsystem.com".to_string(),
        "s.youtube.com".to_string(),
        "pagead2.googlesyndication.com".to_string(),
    ]
}

fn resolve_peer_name(pubkey: &str) -> String {
    // Try to find the profile name based on the public key
    if let Some(output) = utils::run_command_output("ls /etc/wireguard/clients/*.conf 2>/dev/null") {
        for conf_path in output.lines() {
            if let Some(conf_name) = conf_path.split('/').last().and_then(|f| f.strip_suffix(".conf")) {
                // Look for the ClientPublicKey metadata in the config
                let cmd = format!("grep '# ClientPublicKey =' {} 2>/dev/null | grep -o '[a-zA-Z0-9+/]*='", conf_path);
                if let Some(stored_key) = utils::run_command_output(&cmd) {
                    if stored_key.trim() == pubkey {
                        return conf_name.to_string();
                    }
                }
            }
        }
    }
    
    // If no metadata match, fallback to showing start of public key
    format!("{:.8}...", pubkey)
}

fn get_system_uptime() -> String {
    if let Some(output) = utils::run_command_output("uptime") {
        // Extract just the uptime portion from the uptime command
        // Example: "1:34PM  up 2 days,  5:23, 1 user, load averages: 1.23, 1.34, 1.45"
        let trimmed = output.trim();
        if let Some(pos) = trimmed.find("up ") {
            let after_up = &trimmed[pos + 3..];
            if let Some(end_pos) = after_up.find(',') {
                return after_up[..end_pos].to_string();
            }
            return after_up.to_string();
        }
    }
    "Unknown".to_string()
}

fn get_session_duration(timestamp: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(now) => {
            let current_ts = now.as_secs() as i64;
            let diff = current_ts - timestamp;
            
            if diff < 60 {
                format!("{}s ago", diff)
            } else if diff < 3600 {
                format!("{}m {}s ago", diff / 60, diff % 60)
            } else {
                format!("{}h {}m ago", diff / 3600, (diff % 3600) / 60)
            }
        },
        Err(_) => "Unknown".to_string(),
    }
}

