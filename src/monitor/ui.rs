use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, List, ListItem, ListState, Paragraph, Row, Table, TableState, Sparkline},
    Frame,
};
use super::data::{DnsStats, NetStats, get_live_blocked_stats, get_system_uptime};
use super::peers::get_active_peers_with_health;
use crate::utils;

pub fn render_dashboard(f: &mut Frame, n: &NetStats, d: &DnsStats, vpn_table_state: &mut TableState, dns_list_state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),       // Header
            Constraint::Length(3),       // Trends
            Constraint::Length(4),       // DNS Stats
            Constraint::Percentage(30),  // VPN Sessions
            Constraint::Min(15),         // DNS Logs
            Constraint::Length(1),       // Footer
        ]).split(f.size());

    // --- HEADER ---
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" LYNXEDGE ENTERPRISE CORE v4.2", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            Span::styled(format!("  UPTIME: {}", get_system_uptime()), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::raw(" [UNBOUND: "), Span::styled("ACTIVE", Style::default().fg(Color::Green)), Span::raw("] "),
            Span::raw("[WIREGUARD: "), Span::styled("ACTIVE", Style::default().fg(Color::Green)), Span::raw("]"),
        ]),
    ]).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // --- TRENDS (Sparklines) ---
    let trend_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let rx_data: Vec<u64> = n.rx_history.iter().cloned().collect();
    let rx_spark = Sparkline::default()
        .block(Block::default().title(" RX Trend (kbps) ").borders(Borders::ALL))
        .data(&rx_data).style(Style::default().fg(Color::Green));
    f.render_widget(rx_spark, trend_chunks[0]);

    let tx_data: Vec<u64> = n.tx_history.iter().cloned().collect();
    let tx_spark = Sparkline::default()
        .block(Block::default().title(" TX Trend (kbps) ").borders(Borders::ALL))
        .data(&tx_data).style(Style::default().fg(Color::Yellow));
    f.render_widget(tx_spark, trend_chunks[1]);

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

    let rolling_items: Vec<ListItem> = d.blocked_domains.iter().map(|d| ListItem::new(format!(" ✖ {}", d))).collect();
    let rolling_list = List::new(rolling_items).block(Block::default().title(" Recent Blocks ").borders(Borders::ALL));
    f.render_widget(rolling_list, dns_log_chunks[0]);

    let (top_domains, counts) = get_live_blocked_stats();
    let top_items: Vec<ListItem> = top_domains.iter().zip(counts.iter()).map(|(d, c)| {
        ListItem::new(format!(" {:>3}x {}", c, d))
    }).collect();
    let top_list = List::new(top_items).block(Block::default().title(" Top Blocked ").borders(Borders::ALL));
    f.render_widget(top_list, dns_log_chunks[1]);

    let footer = Paragraph::new("[Q] EXIT | [↑↓] SCROLL").style(Style::default().dim());
    f.render_widget(footer, chunks[5]);
}