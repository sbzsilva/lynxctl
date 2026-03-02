use crate::utils;
use std::collections::VecDeque;

const HISTORY_LIMIT: usize = 60;

#[derive(Debug, Default)]
pub struct NetStats {
    pub last_rx: u64,
    pub last_tx: u64,
    pub kbps_rx: u32,
    pub kbps_tx: u32,
    pub rx_history: VecDeque<u64>,
    pub tx_history: VecDeque<u64>,
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
    pub query_history: VecDeque<u64>,
}

pub fn get_net_stats(ifname: &str, stats: &mut NetStats) {
    if let Some(output) = utils::run_command_output(&format!("wg show {} transfer | head -n 1", ifname)) {
        let parts: Vec<&str> = output.split_whitespace().collect();
        if parts.len() >= 3 {
            if let (Ok(rx_val), Ok(tx_val)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>()) {
                stats.kbps_rx = ((rx_val.saturating_sub(stats.last_rx) * 8) / 1024) as u32;
                stats.kbps_tx = ((tx_val.saturating_sub(stats.last_tx) * 8) / 1024) as u32;
                
                stats.last_rx = rx_val;
                stats.last_tx = tx_val;

                stats.rx_history.push_back(stats.kbps_rx as u64);
                stats.tx_history.push_back(stats.kbps_tx as u64);
                if stats.rx_history.len() > HISTORY_LIMIT {
                    stats.rx_history.pop_front();
                    stats.tx_history.pop_front();
                }
            }
        }
    }
}

pub fn get_dns_stats(stats: &mut DnsStats) {
    if let Some(output) = utils::run_command_output("unbound-control stats_noreset") {
        let mut current_total: i32 = 0; // Explicitly typed to i32
        for line in output.lines() {
            if line.starts_with("total.num.queries=") {
                current_total = line.split('=').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            } else if line.starts_with("total.num.cachehits=") {
                stats.cache_hits = line.split('=').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            } else if line.starts_with("num.answer.rcode.NXDOMAIN=") {
                stats.blocked_count = line.split('=').nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            } else if line.starts_with("total.answer.time.avg=") {
                stats.avg_response_time = line.split('=').nth(1)
                    .and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0) * 1000.0;
            }
        }
        
        let delta = current_total.saturating_sub(stats.total_queries);
        stats.query_history.push_back(delta as u64);
        if stats.query_history.len() > HISTORY_LIMIT { stats.query_history.pop_front(); }

        stats.total_queries = current_total;
        if stats.total_queries > 0 {
            stats.hit_rate = (stats.cache_hits * 100) / stats.total_queries;
            stats.block_rate = (stats.blocked_count * 100) / stats.total_queries;
        }
    }
    stats.blocked_domains = get_top_blocked_domains();
}

pub fn get_top_blocked_domains() -> Vec<String> {
    let cmd = "doas grep 'NXDOMAIN' /var/unbound/unbound.log | tail -n 20 | awk '{print $5}'";
    if let Some(output) = utils::run_command_output(cmd) {
        let mut domains: Vec<String> = output.lines()
            .map(|s| s.trim_end_matches('.').to_string())
            .filter(|s| !s.is_empty()).collect();
        domains.reverse();
        domains.dedup();
        return domains;
    }
    vec![]
}

pub fn get_live_blocked_stats() -> (Vec<String>, Vec<u32>) {
    let cmd = "doas grep 'NXDOMAIN' /var/unbound/unbound.log | awk '{print $5}' | sort | uniq -c | sort -nr | head -10";
    if let Some(output) = utils::run_command_output(cmd) {
        let mut domains = Vec::new();
        let mut counts = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(count) = parts[0].parse::<u32>() {
                    counts.push(count);
                    domains.push(parts[1].trim_end_matches('.').to_string());
                }
            }
        }
        return (domains, counts);
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