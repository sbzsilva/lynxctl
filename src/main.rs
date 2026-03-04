#[cfg(not(unix))]
compile_error!("This application is designed to run on Unix-like systems only.");

use std::process;
use clap::{Arg, Command};
use console::style;

mod users;
mod network;
mod system;
mod utils;
mod monitor;

pub const APP_ROOT: &str = "/opt/lynxedge";

fn main() {
    let matches = Command::new("LynxEdge Control Interface")
        .version("5.0")
        .about("Enterprise Appliance Controller for hardened OpenBSD network gateways.")
        .subcommand(
            Command::new("users")
                .about("Identity Management: create, delete, list, qr")
                .arg(Arg::new("action").required(true).index(1))
                .arg(Arg::new("name").index(2)),
        )
        .subcommand(
            Command::new("network")
                .about("Network Logic: live, status, whitelist, netinfo")
                .arg(Arg::new("action").required(true).index(1))
                .arg(Arg::new("domain").index(2)),
        )
        .subcommand(
            Command::new("system")
                .about("Appliance Maintenance: update, upgrade, test, sync, audit")
                .arg(Arg::new("action").required(true).index(1)),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("users", sub_m)) => {
            print_banner();
            let action = sub_m.get_one::<String>("action").unwrap();
            match action.as_str() {
                "list" => users::list_clients(),
                "create" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::create_user(name);
                    } else {
                        eprintln!("{} Error: Missing username.", style("[ERROR]").red());
                        process::exit(1);
                    }
                },
                "delete" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::delete_user(name);
                    } else {
                        eprintln!("{} Error: Missing username.", style("[ERROR]").red());
                        process::exit(1);
                    }
                },
                "qr" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::show_existing_qr(name);
                    } else {
                        eprintln!("{} Error: Missing username.", style("[ERROR]").red());
                        process::exit(1);
                    }
                },
                _ => eprintln!("Invalid user action."),
            }
        },
        Some(("network", sub_m)) => {
            let action = sub_m.get_one::<String>("action").unwrap();
            if action != "live" { print_banner(); }

            match action.as_str() {
                "live" => {
                    if let Err(e) = monitor::run_live_dashboard() {
                        eprintln!("{} Dashboard error: {}", style("[ERROR]").red(), e);
                    }
                },
                "status" => monitor::show_status_dashboard(),
                "netinfo" => system::netinfo(),
                "whitelist" => {
                    if let Some(domain) = sub_m.get_one::<String>("domain") {
                        network::whitelist_domain(domain);
                    } else {
                        eprintln!("{} Error: Missing domain.", style("[ERROR]").red());
                    }
                },
                _ => eprintln!("Invalid network action."),
            }
        },
        Some(("system", sub_m)) => {
            print_banner();
            let action = sub_m.get_one::<String>("action").unwrap();
            match action.as_str() {
                "update" => system::update_ads(),
                "upgrade" => system::upgrade_system(),
                "test" | "audit" => system::run_security_audit(),
                "sync" => system::sync_kernel(),
                _ => eprintln!("Invalid system action."),
            }
        },
        _ => {
            print_banner();
            print_usage();
        }
    }
}

fn print_banner() {
    println!("{}", style("    __                  ____   __          ").cyan());
    println!("{}", style("   / /  __ _____ __ __ / __/__/ /__ ____ ").cyan());
    println!("{}", style("  / /__/ // / _ \\\\ \\ // _// _  / _ `/ -_)").cyan());
    println!("{}", style(" /____/\\_, /_//_/_\\_\\/___/\\_,_/\\_, /\\__/ ").cyan());
    println!("{}", style("      /___/                   /___/      ").cyan());
    println!();
    println!("{} - LynxEdge Appliance Controller", style("LYNXCTL(8)").bold());
}

fn print_usage() {
    println!("Usage: lynxctl [category] [action] [args]\n");
    let categories = [
        ("users", "create, delete, list, qr"),
        ("network", "live, status, whitelist, netinfo"),
        ("system", "update, upgrade, test, sync, audit"),
    ];

    for (cat, actions) in &categories {
        println!("  {}: {}", style(cat).yellow(), style(actions).green());
    }
}