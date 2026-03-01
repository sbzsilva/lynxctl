pub mod data;
pub mod peers;
pub mod ui;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, widgets::{ListState, TableState}, Terminal};
use std::{io, time::{Duration, Instant}};

use data::{DnsStats, NetStats, get_dns_stats, get_live_blocked_stats, get_net_stats};

pub fn run_live_dashboard() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut n = NetStats::default();
    let mut d = DnsStats::default();
    let mut vpn_table_state = TableState::default();
    let mut dns_list_state = ListState::default();

    let tick_rate = Duration::from_millis(1000);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            ui::render_dashboard(f, &n, &d, &mut vpn_table_state, &mut dns_list_state);
        })?;

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or(Duration::ZERO);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        let len = d.blocked_domains.len();
                        if len > 0 {
                            let next = dns_list_state.selected().map(|i| (i + 1) % len).unwrap_or(0);
                            dns_list_state.select(Some(next));
                        }
                    }
                    KeyCode::Up => {
                        let len = d.blocked_domains.len();
                        if len > 0 {
                            let prev = dns_list_state.selected()
                                .map(|i| if i == 0 { len - 1 } else { i - 1 })
                                .unwrap_or(0);
                            dns_list_state.select(Some(prev));
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            get_net_stats("wg0", &mut n);
            get_dns_stats(&mut d);

            let (domains, _) = get_live_blocked_stats();
            if !domains.is_empty() {
                dns_list_state.select(Some(domains.len() - 1));
            }
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

pub fn show_status_dashboard() {
    let mut d = DnsStats::default();
    get_dns_stats(&mut d);
    println!("\n{} Status", console::style("LynxEdge Enterprise").bold());
    println!("{}", "═".repeat(40));
    println!("System Uptime:  {}", data::get_system_uptime());
    println!("Total Queries:  {}", d.total_queries);
    println!("Block Rate:     {}%", d.block_rate);
}