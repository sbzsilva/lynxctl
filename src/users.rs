use std::fs;
use std::path::Path;
use anyhow::Result;
use console::style;

use crate::{WG_GW, utils};

pub fn create_user(name: &str) {
    match get_next_available_ip() {
        Ok(ip) => {
            println!("{} Generating keys and assigning IP: {}", 
                style("[INFO]").color256(24), ip);
            
            // Get server public key
            let server_pub_result = utils::run_command_output("wg show wg0 public-key");
                
            let server_pub = match server_pub_result {
                Some(output) => output.trim().to_string(),
                None => {
                    eprintln!("{} Failed to get server public key", 
                        style("[ERROR]").red());
                    return;
                }
            };
            
            // Get server WAN IP
            let server_ip_result = utils::run_command_output("curl -s ifconfig.me");
                
            let server_ip = match server_ip_result {
                Some(output) => output.trim().to_string(),
                None => {
                    eprintln!("{} Failed to get server IP", 
                        style("[ERROR]").red());
                    return;
                }
            };
            
            // Generate client config
            let client_config = format!(
                "# ClientPublicKey = $pub\n\
                [Interface]\n\
                PrivateKey = $priv\n\
                Address = {}/32\n\
                DNS = {}\n\n\
                [Peer]\n\
                PublicKey = {}\n\
                Endpoint = {}:51820\n\
                AllowedIPs = 0.0.0.0/0\n",
                ip, WG_GW, server_pub, server_ip
            );
            
            // Create the command to generate and save the config
            let cmd = format!(
                "priv=$(wg genkey); pub=$(echo $priv | wg pubkey); \
                printf '{}' > /etc/wireguard/clients/{}.conf && \
                wg set wg0 peer \"$pub\" allowed-ips {}/32",
                client_config.replace("'", "'\"'\"'"), name, ip
            );
            
            if utils::run_command(&cmd) {
                println!("{} Profile for {} created.", 
                    style("[SUCCESS]").green(), name);
                show_qr(name);
            } else {
                eprintln!("{} Failed to create WireGuard profile.", 
                    style("[ERROR]").red());
            }
        },
        Err(e) => {
            eprintln!("{} No available IPs: {}", 
                style("[ERROR]").red(), e);
        }
    }
}

fn get_next_available_ip() -> Result<String> {
    // In a real implementation, this would scan existing configs to find an available IP
    // For now, we'll simulate finding an IP
    for i in 2..255 {
        let target = format!("10.200.200.{}", i);
        
        // Check if this IP is already used in any client config
        let clients_dir = Path::new("/etc/wireguard/clients");
        if clients_dir.exists() {
            if let Ok(entries) = fs::read_dir(clients_dir) {
                let mut found = false;
                for entry in entries.flatten() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if filename.ends_with(".conf") {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                if content.contains(&target) {
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                
                if !found {
                    return Ok(target);
                }
            }
        } else {
            // If directory doesn't exist, return the first IP
            return Ok(target);
        }
    }
    
    Err(anyhow::anyhow!("No available IPs"))
}

pub fn delete_user(name: &str) {
    let path = format!("/etc/wireguard/clients/{}.conf", name);
    
    match fs::remove_file(&path) {
        Ok(_) => {
            println!("{} Deleted {}.conf", style("Deleted").red(), name);
            crate::system::sync_kernel(); // Sync after deletion
        },
        Err(e) => eprintln!("Delete failed: {}", e),
    }
}

pub fn list_clients() {
    println!();
    println!(" {:<15} {:<15} {:<15}", 
        style("Profile").bold(), 
        style("Status").bold(), 
        style("IP Address").bold());
    println!("{}", "─".repeat(50));

    let clients_dir = Path::new("/etc/wireguard/clients");
    if !clients_dir.exists() {
        eprintln!("{} Directory /etc/wireguard/clients does not exist", style("[ERROR]").red());
        return;
    }

    if let Ok(entries) = fs::read_dir(clients_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".conf") {
                    let name = filename.strip_suffix(".conf").unwrap_or(filename);
                    
                    // Extract IP from config file
                    let mut ip = "Unknown".to_string();
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        for line in content.lines() {
                            if line.starts_with("Address = ") {
                                // Extract IP without /32 suffix
                                if let Some(addr_part) = line.strip_prefix("Address = ") {
                                    ip = addr_part.split('/').next().unwrap_or("Unknown").to_string();
                                    break;
                                }
                            }
                        }
                    }
                    
                    println!("{:<15} {:<15} {:<15}", 
                        name, 
                        style("AVAILABLE").green(), 
                        ip);
                }
            }
        }
    } else {
        eprintln!("{} Could not read clients directory", style("[ERROR]").red());
    }
}

pub fn show_qr(name: &str) {
    let cmd = format!("qrencode -t ansiutf8 < /etc/wireguard/clients/{}.conf", name);
    
    if !utils::run_command(&cmd) {
        eprintln!("Failed to display QR code for {}", name);
    }
}