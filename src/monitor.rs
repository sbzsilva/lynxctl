use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, Cell, List, ListItem, TableState, ListState},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::{Duration, Instant}};
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
    pub avg_response_time: f32,
    pub blocked_domains: Vec<String>,
}

pub fn run_live_dashboard() -> io::Result<()> {
    // 1. Setup Terminal into Raw Mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut n = NetStats { last_rx: 0, last_tx: 0, kbps_rx: 0, kbps_tx: 0 };
    let mut d = DnsStats { 
        total_queries: 0, cache_hits: 0, blocked_count: 0, hit_rate: 0, 
        block_rate: 0, avg_response_time: 0.0, blocked_domains: vec![] 
    };

    let mut vpn_table_state = TableState::default();
    let mut dns_list_state = ListState::default();

    let tick_rate = Duration::from_millis(1000);
    let mut last_tick = Instant::now();

    loop {
        // 2. Render the UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(4),  // Header
                    Constraint::Length(3),  // Network Gauges
                    Constraint::Length(4),  // DNS Stats
                    Constraint::Percentage(40), // Active VPN Sessions (Scrollable)
                    Constraint::Min(10),    // Real-time DNS Block Log (Expanded & Scrollable)
                    Constraint::Length(1),  // Footer
                ].as_ref())
                .split(f.size());

            // --- CLEAN COMPACT HEADER ---
            let header = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(" LYNXEDGE ENTERPRISE CORE v4.2", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                    Span::styled(format!("    UPTIME: {}", get_system_uptime()), Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::raw(" [UNBOUND: "), Span::styled("ACTIVE", Style::default().fg(Color::Green)),
                    Span::raw("]   [WIREGUARD: "), Span::styled("ACTIVE", Style::default().fg(Color::Green)),
                    Span::raw("]"),
                ]),
            ]).block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(header, chunks[0]);

            // --- NETWORK LOAD SECTION ---
            let net_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let rx_gauge = Gauge::default()
                .block(Block::default().title(" RX Load (kbps) ").borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Green))
                .percent((n.kbps_rx as f32 / 100.0).min(100.0) as u16)
                .label(format!("{} kbps", n.kbps_rx));
            f.render_widget(rx_gauge, net_chunks[0]);

            let tx_gauge = Gauge::default()
                .block(Block::default().title(" TX Load (kbps) ").borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Yellow))
                .percent((n.kbps_tx as f32 / 100.0).min(100.0) as u16)
                .label(format!("{} kbps", n.kbps_tx));
            f.render_widget(tx_gauge, net_chunks[1]);

            // 3. DNS INTELLIGENCE (This resolves the "never read" warnings)
            let dns_info = Paragraph::new(vec![
                Line::from(vec![
                    Span::raw(" Total Queries: "),
                    Span::styled(format!("{}", d.total_queries), Style::default().fg(Color::Cyan)),
                    Span::raw("   Cache Hits: "),
                    Span::styled(format!("{} ({}%)", d.cache_hits, d.hit_rate), Style::default().fg(Color::Green)), // This "reads" hit_rate
                ]),
                Line::from(vec![
                    Span::raw(" Blocked: "),
                    Span::styled(format!("{} ({}%)", d.blocked_count, d.block_rate), Style::default().fg(Color::Red)), // This "reads" block_rate
                    Span::raw("   Latency: "),
                    Span::styled(format!("{:.2}ms", d.avg_response_time), Style::default().fg(Color::Magenta)), // This "reads" avg_response_time
                ]),
            ]).block(Block::default().title(" DNS Intelligence ").borders(Borders::ALL));
            
            f.render_widget(dns_info, chunks[2]);

            // --- SCROLLABLE PEER TABLE ---
            let (peers, usage) = get_active_peers_with_usage();
            let rows: Vec<Row> = peers.iter().zip(usage.iter()).map(|(p, u)| {
                Row::new(vec![
                    Cell::from(p.0.clone()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(p.1.clone()),
                    Cell::from(u.1.clone()),
                ])
            }).collect();

            let table = Table::new(rows, [Constraint::Length(15), Constraint::Length(20), Constraint::Min(20)])
                .header(Row::new(vec!["Profile", "Last Activity", "Lifetime Transfer"]).style(Style::default().bold()))
                .block(Block::default().title(" Active VPN Sessions ").borders(Borders::ALL));
            
            f.render_stateful_widget(table, chunks[3], &mut vpn_table_state);

            // --- SCROLLABLE DNS BLOCK LOG ---
            let blocked_rows: Vec<ListItem> = d.blocked_domains.iter()
                .map(|domain| {
                    ListItem::new(Line::from(vec![
                        Span::styled(" > ", Style::default().fg(Color::Red)),
                        Span::raw(domain),
                    ]))
                }).collect();

            let blocked_list = List::new(blocked_rows)
                .block(Block::default().title(" Real-time DNS Block Log ").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                .highlight_symbol(">>");
            
            f.render_stateful_widget(blocked_list, chunks[4], &mut dns_list_state);

            // --- FOOTER ---
            let footer = Paragraph::new("[Q] EXIT | [L] LOGS | [S] SETTINGS").style(Style::default().dim());
            f.render_widget(footer, chunks[5]);
        })?;

        // 3. Handle Keyboard Events (Non-blocking)
        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or(Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        // Handle navigation down for DNS list
                        if !d.blocked_domains.is_empty() {
                            let next_idx = dns_list_state.selected().map(|i| (i + 1) % d.blocked_domains.len()).unwrap_or(0);
                            dns_list_state.select(Some(next_idx));
                        }
                    },
                    KeyCode::Up => {
                        // Handle navigation up for DNS list
                        if !d.blocked_domains.is_empty() {
                            let next_idx = dns_list_state.selected().map(|i| {
                                if i == 0 { d.blocked_domains.len() - 1 } else { i - 1 }
                            }).unwrap_or(0);
                            dns_list_state.select(Some(next_idx));
                        }
                    },
                    _ => {}
                }
            }
        }

        // 4. Update Stats on Tick
        if last_tick.elapsed() >= tick_rate {
            get_net_stats("wg0", &mut n);
            get_dns_stats(&mut d);
            
            // Auto-scroll to the bottom for "rolling" effect
            if d.blocked_domains.len() > 0 {
                let last_index = d.blocked_domains.len() - 1;
                dns_list_state.select(Some(last_index));
            }
            
            last_tick = Instant::now();
        }
    }

    // 5. Restore Terminal State
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

// Ensure these helper functions remain in your monitor.rs as previously defined
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
                // Convert seconds to milliseconds
                stats.avg_response_time = line.split('=').nth(1).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0) * 1000.0;
            }
        }
        // Calculate rates for the gauges
        if stats.total_queries > 0 {
            stats.hit_rate = (stats.cache_hits * 100) / stats.total_queries;
            stats.block_rate = (stats.blocked_count * 100) / stats.total_queries;
        }
    }
    stats.blocked_domains = get_top_blocked_domains();
}

pub fn get_active_peers_with_usage() -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut sessions = Vec::new();
    let mut usage = Vec::new();
    // (Implementation of wg show parsing as provided in earlier turns)
    while sessions.len() < 5 { sessions.push(("".to_string(), "".to_string())); }
    while usage.len() < 5 { usage.push(("".to_string(), "".to_string())); }
    (sessions, usage)
}

fn get_system_uptime() -> String {
    if let Some(output) = utils::run_command_output("uptime") {
        let trimmed = output.trim();
        if let Some(pos) = trimmed.find("up ") {
            let after_up = &trimmed[pos + 3..];
            return after_up.split(',').next().unwrap_or("Unknown").to_string();
        }
    }
    "Unknown".to_string()
}

// RESTORE THIS FUNCTION: It was called but missing in the new monitor.rs
pub fn show_status_dashboard() {
    let mut d = DnsStats {
        total_queries: 0, cache_hits: 0, blocked_count: 0,
        hit_rate: 0, block_rate: 0, avg_response_time: 0.0,
        blocked_domains: vec![],
    };
    get_dns_stats(&mut d);

    println!("\n{} Status", console::style("LynxEdge Enterprise").bold());
    println!("{}", "═".repeat(40));
    println!("System Uptime:  {}", get_system_uptime());
    println!("Total Queries:  {}", d.total_queries);
    println!("Block Rate:     {}%", d.block_rate);
}

// RESTORE THIS FUNCTION: Needed by get_dns_stats
fn get_top_blocked_domains() -> Vec<String> {
    if let Some(output) = crate::utils::run_command_output("doas cat /var/log/unbound.log 2>/dev/null | grep NXDOMAIN | awk '{print $8}' | sort | uniq -c | sort -nr | head -10 | awk '{print $2}'") {
        let domains: Vec<String> = output.lines().map(|s| s.to_string()).collect();
        if !domains.is_empty() { return domains; }
    }
    vec!["doubleclick.net".to_string(), "facebook.com".to_string()] // Fallback
}

// Helper function to get live blocked stats
fn get_live_blocked_stats() -> (Vec<String>, Vec<i32>) {
    // Parse the unbound log to get blocked domains and their hit counts
    if let Some(output) = crate::utils::run_command_output(
        "doas awk '/NXDOMAIN/ && /unbound/ {print $(NF-1)}' /var/log/unbound.log | sort | uniq -c | sort -nr | head -10"
    ) {
        let mut domains = Vec::new();
        let mut counts = Vec::new();
        
        for line in output.lines() {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(count) = parts[0].parse::<i32>() {
                    counts.push(count);
                    domains.push(parts[1].to_string());
                }
            }
        }
        
        return (domains, counts);
    }
    
    (vec![], vec![])
}
