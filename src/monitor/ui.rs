use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize}, // Added Stylize trait
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Sparkline}, // Removed unused Gauge
    Frame,
};
use super::data::{DnsStats, NetStats, get_live_blocked_stats, get_system_uptime};
use super::peers::get_active_peers_with_health;
// Removed unused crate::utils

pub fn render_dashboard(f: &mut Frame, n: &NetStats, d: &DnsStats, vpn_table_state: &mut TableState, dns_list_state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),       // Header (Reduced to bottom border only)
            Constraint::Length(6),       // Traffic Card (New nested area)
            Constraint::Length(4),       // DNS Stats
            Constraint::Percentage(30),  // VPN Sessions
            Constraint::Min(10),         // DNS Logs
            Constraint::Length(1),       // Footer
        ]).split(f.size());

    // --- MODERN HEADER ---
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" LYNXEDGE CORE", Style::default().bold().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled(format!("UPTIME: {}", get_system_uptime()), Style::default().dim()),
    ])).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(header, chunks[0]);

    // --- NESTED TRAFFIC CARD ---
    let traffic_block = Block::default().title(" Traffic (wg0) ").borders(Borders::TOP).border_style(Style::default().dim());
    let inner_traffic = traffic_block.inner(chunks[1]);
    f.render_widget(traffic_block, chunks[1]);

    let traffic_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(30)])
        .split(inner_traffic);

    // Left: Sparklines
    let spark_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(traffic_split[0]);

    let rx_data: Vec<u64> = n.rx_history.iter().cloned().collect();
    let rx_spark = Sparkline::default()
        .data(&rx_data)
        .style(Style::default().fg(Color::Green))
        .max(100_000); // Scaled for 100Mbps
    f.render_widget(rx_spark, spark_rows[0]);

    let tx_data: Vec<u64> = n.tx_history.iter().cloned().collect();
    let tx_spark = Sparkline::default()
        .data(&tx_data)
        .style(Style::default().fg(Color::Yellow))
        .max(100_000);
    f.render_widget(tx_spark, spark_rows[1]);

    // Right: Now/Avg/Peak Stats
    let avg_rx = if !n.rx_history.is_empty() {
        (n.rx_history.iter().sum::<u64>() / n.rx_history.len() as u64) as u32
    } else {
        0
    };
    let peak_rx = n.rx_history.iter().max().copied().unwrap_or(0) as u32;
    let avg_tx = if !n.tx_history.is_empty() {
        (n.tx_history.iter().sum::<u64>() / n.tx_history.len() as u64) as u32
    } else {
        0
    };
    let peak_tx = n.tx_history.iter().max().copied().unwrap_or(0) as u32;
    
    let stats_text = vec![
        Line::from(vec![Span::raw("RX "), Span::styled(format!("{:>6} kbps", n.kbps_rx), Style::default().fg(Color::Green).bold())]),
        Line::from(vec![Span::styled(format!("   Avg: {:>6} Peak: {:>6}", avg_rx, peak_rx), Style::default().dim())]),
        Line::from(vec![Span::raw("TX "), Span::styled(format!("{:>6} kbps", n.kbps_tx), Style::default().fg(Color::Yellow).bold())]),
        Line::from(vec![Span::styled(format!("   Avg: {:>6} Peak: {:>6}", avg_tx, peak_tx), Style::default().dim())]),
    ];
    f.render_widget(Paragraph::new(stats_text), traffic_split[1]);

    // --- DNS STATS ---
    let dns_info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw(" Total Queries: "), Span::styled(format!("{}", d.total_queries), Style::default().fg(Color::Cyan)),
            Span::raw("  Cache Hits: "), Span::styled(format!("{} ({}%)", d.cache_hits, d.hit_rate), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw(" Blocked: "), Span::styled(format!("{} ({}%)", d.blocked_count, d.block_rate), Style::default().fg(Color::Red)),
            Span::raw("  Latency: "), Span::styled(format!("{:.2}ms", d.avg_response_time), Style::default().fg(Color::Magenta)),
        ]),
    ]).block(Block::default().title(" DNS Intelligence ").borders(Borders::ALL));
    f.render_widget(dns_info, chunks[2]);

    // --- VPN TABLE ---
    let peers = get_active_peers_with_health();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let rows: Vec<Row> = peers.iter().map(|(name, end, trans, hand)| {
        let age = now.saturating_sub(*hand);
        let color = if age < 180 { Color::Green } else if age < 3600 { Color::Yellow } else { Color::Red };
        Row::new(vec![
            Cell::from(name.clone()).style(Style::default().fg(Color::Cyan)),
            Cell::from(if *hand == 0 { "Never".into() } else { format!("{}s ago", age) }).style(Style::default().fg(color)),
            Cell::from(end.clone()).style(Style::default().dim()),
            Cell::from(trans.clone()),
        ])
    }).collect();

    let table = Table::new(rows, [Constraint::Length(15), Constraint::Length(15), Constraint::Length(25), Constraint::Min(20)])
        .header(Row::new(vec!["Profile", "Handshake", "Endpoint", "Transfer"]).style(Style::default().bold()))
        .block(Block::default().title(" Active VPN Sessions ").borders(Borders::ALL));
    f.render_stateful_widget(table, chunks[3], vpn_table_state);

    // --- DUAL DNS PANELS ---
    let dns_log_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[4]);

    let rolling_items: Vec<ListItem> = d.blocked_domains.iter()
        .map(|d| ListItem::new(format!(" ✖ {}", d)))
        .collect();

    let rolling_list = List::new(rolling_items)
        .block(Block::default().title(" Recent Blocks ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");

    // FIX: Use render_stateful_widget and pass the state
    f.render_stateful_widget(rolling_list, dns_log_chunks[0], dns_list_state);

    let (top_domains, counts) = get_live_blocked_stats();
    let top_items: Vec<ListItem> = top_domains.iter().zip(counts.iter())
        .map(|(d, c)| ListItem::new(format!(" {:>3}x {}", c, d)))
        .collect();

    let top_list = List::new(top_items)
        .block(Block::default().title(" Top Blocked ").borders(Borders::ALL));

    // For the Top 10 list, you can either pass a second state or just render it statically
    f.render_widget(top_list, dns_log_chunks[1]);

    let footer = Paragraph::new("[Q] EXIT | [↑↓] SCROLL").style(Style::default().dim());
    f.render_widget(footer, chunks[5]);
}