use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, List, ListItem, ListState, Paragraph, Row, Table, TableState},
    Frame,
};
use super::data::{DnsStats, NetStats, get_live_blocked_stats, get_system_uptime};
use super::peers::get_active_peers_with_usage;
use crate::utils;

pub fn render_dashboard(
    f: &mut Frame,
    n: &NetStats,
    d: &DnsStats,
    vpn_table_state: &mut TableState,
    dns_list_state: &mut ListState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),       // Header
            Constraint::Length(3),       // Network Gauges
            Constraint::Length(4),       // DNS Stats
            Constraint::Percentage(30),  // Active VPN Sessions
            Constraint::Min(15),         // DNS Block Log
            Constraint::Length(1),       // Footer
        ].as_ref())
        .split(f.size());

    // --- HEADER ---
    let unbound_status = if utils::is_service_running("unbound") {
        Span::styled("ACTIVE", Style::default().fg(Color::Green))
    } else {
        Span::styled("INACTIVE", Style::default().fg(Color::Red))
    };
    let wg_status = if utils::run_command_output("ifconfig wg0 2>/dev/null | grep -q UP && echo ok").is_some() {
        Span::styled("ACTIVE", Style::default().fg(Color::Green))
    } else {
        Span::styled("INACTIVE", Style::default().fg(Color::Red))
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" LYNXEDGE ENTERPRISE CORE v4.2", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
            Span::styled(format!("  UPTIME: {}", get_system_uptime()), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::raw(" [UNBOUND: "), unbound_status, Span::raw("]"),
            Span::raw(" [WIREGUARD: "), wg_status, Span::raw("]"),
        ]),
    ]).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // --- NETWORK GAUGES ---
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

    // --- DNS INTELLIGENCE ---
    let dns_info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw(" Total Queries: "),
            Span::styled(format!("{}", d.total_queries), Style::default().fg(Color::Cyan)),
            Span::raw("  Cache Hits: "),
            Span::styled(format!("{} ({}%)", d.cache_hits, d.hit_rate), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw(" Blocked: "),
            Span::styled(format!("{} ({}%)", d.blocked_count, d.block_rate), Style::default().fg(Color::Red)),
            Span::raw("  Latency: "),
            Span::styled(format!("{:.2}ms", d.avg_response_time), Style::default().fg(Color::Magenta)),
        ]),
    ]).block(Block::default().title(" DNS Intelligence ").borders(Borders::ALL));
    f.render_widget(dns_info, chunks[2]);

    // --- VPN SESSIONS TABLE ---
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
    f.render_stateful_widget(table, chunks[3], vpn_table_state);

    // --- DNS BLOCK LOG ---
    let (blocked_domains, counts) = get_live_blocked_stats();
    let blocked_rows: Vec<ListItem> = blocked_domains.iter().zip(counts.iter()).map(|(domain, count)| {
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {:>3}x ", count), Style::default().fg(Color::Gray)),
            Span::styled(" ✖ ", Style::default().fg(Color::Red)),
            Span::raw(domain.clone()),
        ]))
    }).collect();

    let blocked_list = List::new(blocked_rows)
        .block(Block::default().title(" Real-time DNS Block Log (Rolling) ").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
        .highlight_symbol(">> ");
    f.render_stateful_widget(blocked_list, chunks[4], dns_list_state);

    // --- FOOTER ---
    let footer = Paragraph::new("[Q] EXIT | [↑↓] SCROLL DNS LOG").style(Style::default().dim());
    f.render_widget(footer, chunks[5]);
}