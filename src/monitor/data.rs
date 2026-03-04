use std::collections::VecDeque;
use crate::{utils, APP_ROOT}; // Added APP_ROOT reference

const HISTORY_LIMIT: usize = 120;

// ... (NetStats and DnsStats structs remain the same)

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
    // Unbound-control uses the symlinked path automatically via its default config
    if let Some(output) = utils::run_command_output("unbound-control stats_noreset") {
        let mut current_total: i32 = 0;
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
    // UPDATED: Points to appliance log path
    let cmd = format!("doas grep 'NXDOMAIN' {}/logs/unbound.log | tail -n 20 | awk '{{print $5}}'", APP_ROOT);
    if let Some(output) = utils::run_command_output(&cmd) {
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
    // UPDATED: Points to appliance log path
    let cmd = format!("doas grep 'NXDOMAIN' {}/logs/unbound.log | awk '{{print $5}}' | sort | uniq -c | sort -nr | head -10", APP_ROOT);
    if let Some(output) = utils::run_command_output(&cmd) {
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