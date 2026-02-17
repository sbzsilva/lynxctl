#[cfg(not(unix))]
compile_error!("This application is designed to run on Unix-like systems only.");

use std::process;
use clap::{Arg, Command};
use console; // 保留 console crate 用于样式打印，但移除未使用的 Style

mod users;
mod network;
mod system;
mod utils;
mod monitor;

const WG_GW: &str = "10.200.200.1";

fn main() {
    let matches = Command::new("LynxEdge Control Interface")
        .version("4.0")
        .about("A high-performance, consolidated C-core management suite for secure network gateways on OpenBSD.")
        .subcommand(
            Command::new("users")
                .about("User management: create, delete, list, qr")
                .arg(Arg::new("action")
                    .required(true)
                    .index(1)
                    .help("Action to perform: create, delete, list, qr"))
                .arg(Arg::new("name")
                    .index(2)
                    .help("Username for create/delete/qr actions")),
        )
        .subcommand(
            Command::new("network")
                .about("Network operations: live, status, whitelist, netinfo")
                .arg(Arg::new("action")
                    .required(true)
                    .index(1)
                    .help("Action to perform: live, status, whitelist, netinfo"))
                .arg(Arg::new("domain")
                    .index(2)
                    .help("Domain for whitelist action")),
        )
        .subcommand(
            Command::new("system")
                .about("System operations: update, upgrade, test, sync")
                .arg(Arg::new("action")
                    .required(true)
                    .index(1)
                    .help("Action to perform: update, upgrade, test, sync")),
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
                        eprintln!("Invalid user action or missing name.");
                        process::exit(1);
                    }
                },
                "delete" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::delete_user(name);
                    } else {
                        eprintln!("Invalid user action or missing name.");
                        process::exit(1);
                    }
                },
                "qr" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::show_qr(name);
                    } else {
                        eprintln!("Invalid user action or missing name.");
                        process::exit(1);
                    }
                },
                _ => eprintln!("Invalid user action or missing name."),
            }
        },
        Some(("network", sub_m)) => {
            print_banner();
            let action = sub_m.get_one::<String>("action").unwrap();
            match action.as_str() {
                "live" => network::run_live_dashboard(),
                "status" => network::show_status_dashboard(),
                "netinfo" => system::netinfo(),
                "whitelist" => {
                    if let Some(domain) = sub_m.get_one::<String>("domain") {
                        network::whitelist_domain(domain);
                    } else {
                        eprintln!("Invalid network action.");
                        process::exit(1);
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
                "test" => network::test_blocking(),
                "sync" => system::sync_kernel(),
                _ => eprintln!("Invalid system action."),
            }
        },
        _ => {
            print_banner();
            print_usage();
        }
    }
        Some(("users", sub_m)) => {
            let action = sub_m.get_one::<String>("action").unwrap();
            match action.as_str() {
                "list" => users::list_clients(),
                "create" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::create_user(name);
                    } else {
                        eprintln!("Invalid user action or missing name.");
                        process::exit(1);
                    }
                },
                "delete" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::delete_user(name);
                    } else {
                        eprintln!("Invalid user action or missing name.");
                        process::exit(1);
                    }
                },
                "qr" => {
                    if let Some(name) = sub_m.get_one::<String>("name") {
                        users::show_qr(name);
                    } else {
                        eprintln!("Invalid user action or missing name.");
                        process::exit(1);
                    }
                },
                _ => eprintln!("Invalid user action or missing name."),
            }
        },
        Some(("network", sub_m)) => {
            let action = sub_m.get_one::<String>("action").unwrap();
            match action.as_str() {
                "live" => network::run_live_dashboard(),
                "status" => network::show_status_dashboard(),
                "netinfo" => system::netinfo(),
                "whitelist" => {
                    if let Some(domain) = sub_m.get_one::<String>("domain") {
                        network::whitelist_domain(domain);
                    } else {
                        eprintln!("Invalid network action.");
                        process::exit(1);
                    }
                },
                _ => eprintln!("Invalid network action."),
            }
        },
        Some(("system", sub_m)) => {
            let action = sub_m.get_one::<String>("action").unwrap();
            match action.as_str() {
                "update" => system::update_ads(),
                "upgrade" => system::upgrade_system(),
                "test" => network::test_blocking(),
                "sync" => system::sync_kernel(),
                _ => eprintln!("Invalid system action."),
            }
        },
        _ => {
            print_usage();
        }
    }
}

fn print_banner() {
    println!("{}", console::style("    __                ____   __          ").cyan());
    println!("{}", console::style("   / /  __ _____ __ __ / __/__/ /__ ____ ").cyan());
    println!("{}", console::style("  / /__/ // / _ \\\\ \\ // _// _  / _ `/ -_)").cyan());
    println!("{}", console::style(" /____/\\_, /_//_/_\\_\\/___/\\_,_/\\_, /\\__/ ").cyan());
    println!("{}", console::style("      /___/                   /___/      ").cyan());
    println!();
    println!("{} - LynxEdge Control Interface", 
        console::style("LYNXCTL(8)").bold());
}

fn print_usage() {
    println!("LYNXCTL(8) - LynxEdge Control Interface");
    println!("Usage: lynxctl [category] [action] [args]\n");

    let categories = [
        ("users", "create, delete, list, qr"),
        ("network", "live, status, whitelist, netinfo"),
        ("system", "update, upgrade, test, sync"),
    ];

    for (name, actions) in categories {
        println!("  - {:<10}:  {}", console::style(name).cyan(), actions); 
    }
}