use crate::utils;

#[derive(Debug, Default)]
pub struct NetStats {
    pub last_rx: u64,
    pub last_tx: u64,
    pub kbps_rx: u32,
    pub kbps_tx: u32,
}

#[derive(Debug, Default)]
pub struct DnsStats {
    pub total_queries: i32,
    pub cache_hits: i32,
    pub blocked_count: i32,
    pub hit_rate: i32,
    pub block_rate: i32,
    pub avg_response_time: f32,
    pub blocked_domains: Vec<String>,
}

pub fn get_net_stats(ifname: &str, stats: &mut NetStats) {
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

pub fn get_dns_stats(stats: &mut DnsStats) {
    if let Some(output) = utils::run_command_output("unbound-control stats_noreset") {
        for line in output.lines() {
            if line.starts_with("total.num.queries=") {
                stats.total_queries = line.split('=').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            } else if line.starts_with("total.num.cachehits=") {
                stats.cache_hits = line.split('=').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            } else if line.starts_with("num.answer.rcode.NXDOMAIN=") {
                stats.blocked_count = line.split('=').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            } else if line.starts_with("total.answer.time.avg=") {
                stats.avg_response_time = line.split('=').nth(1)
                    .and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0) * 1000.0;
            }
        }
        if stats.total_queries > 0 {
            stats.hit_rate = (stats.cache_hits * 100) / stats.total_queries;
            stats.block_rate = (stats.blocked_count * 100) / stats.total_queries;
        }
    }
    stats.blocked_domains = get_top_blocked_domains();
}

pub fn get_top_blocked_domains() -> Vec<String> {
    let cmd = "doas grep 'NXDOMAIN' /var/unbound/unbound.log \
               | awk '{print $7}' | sort | uniq -c | sort -nr \
               | head -10 | awk '{print $2}'";
    if let Some(output) = utils::run_command_output(cmd) {
        let domains: Vec<String> = output
            .lines()
            .map(|s| s.trim_end_matches('.').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !domains.is_empty() {
            return domains;
        }
    }
    vec![]
}

pub fn get_live_blocked_stats() -> (Vec<String>, Vec<u32>) {
    let cmd = "doas grep 'NXDOMAIN' /var/unbound/unbound.log \
               | awk '{print $7}' | sort | uniq -c | sort -nr | head -10";
    if let Some(output) = utils::run_command_output(cmd) {
        let mut domains = Vec::new();
        let mut counts = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
            if parts.len() == 2 {
                if let Ok(count) = parts[0].trim().parse::<u32>() {
                    counts.push(count);
                    domains.push(parts[1].trim().trim_end_matches('.').to_string());
                }
            }
        }
        if !domains.is_empty() {
            return (domains, counts);
        }
    }
    (vec![], vec![])
}

pub fn get_system_uptime() -> String {
    if let Some(output) = utils::run_command_output("uptime") {
        let trimmed = output.trim();
        if let Some(pos) = trimmed.find("up ") {
            let after_up = &trimmed[pos + 3..];
            return after_up.split(',').next().unwrap_or("Unknown").to_string();
        }
    }
    "Unknown".to_string()
}