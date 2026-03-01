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
            
            // Generate client config template
            let client_config = format!(
                "[Interface]\n\
                PrivateKey = $priv\n\
                Address = {}/32\n\
                DNS = {}\n\n\
                [Peer]\n\
                PublicKey = {}\n\
                Endpoint = {}:51820\n\
                AllowedIPs = 0.0.0.0/0\n",
                ip, WG_GW, server_pub, server_ip
            );
            
            // Use doas to write the file to the restricted directory and update the kernel
            let cmd = format!(
                "priv=$(wg genkey); pub=$(echo $priv | wg pubkey); \
                config_content=\"$(echo '{}' | sed \"s/\\$priv/$priv/\")\"; \
                echo \"$config_content\" | doas tee /etc/wireguard/clients/{}.conf > /dev/null && \
                doas wg set wg0 peer \"$pub\" allowed-ips {}/32",
                client_config, name, ip
            );
            
            if utils::run_command(&cmd) {
                println!("{} Profile for {} created.", 
                    style("[SUCCESS]").green(), name);
                // Immediately call show_qr to display the code
                show_qr(name);
            } else {
                eprintln!("{} Failed to create WireGuard profile. Check doas permissions.", 
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
    for i in 2..255 {
        let target = format!("10.200.200.{}", i);
        
        // Use doas to check directory existence and file contents due to restricted permissions
        let check_cmd = format!("doas grep -r \"{}\" /etc/wireguard/clients/", target);
        if !utils::run_command(&check_cmd) {
            return Ok(target);
        }
    }
    
    Err(anyhow::anyhow!("No available IPs"))
}

pub fn delete_user(name: &str) {
    let path = format!("/etc/wireguard/clients/{}.conf", name);
    // Use doas to remove the file since it's in a root-owned directory
    let cmd = format!("doas rm {}", path);
    
    if utils::run_command(&cmd) {
        println!("{} Deleted {}.conf", style("Deleted").red(), name);
        crate::system::sync_kernel(); 
    } else {
        eprintln!("Delete failed for {}", name);
    }
}

pub fn list_clients() {
    println!();
    println!(" {:<15} {:<15} {:<15}", 
        style("Profile").bold(), 
        style("Status").bold(), 
        style("IP Address").bold());
    println!("{}", "─".repeat(50));

    // Use doas to list files to bypass permission denied errors
    let cmd = "doas ls /etc/wireguard/clients/*.conf 2>/dev/null || true";
    if let Some(output) = utils::run_command_output(cmd) {
        if output.trim().is_empty() {
            eprintln!("{} No client configurations found.", style("[ERROR]").red());
            return;
        }
        
        for path in output.lines() {
            let name = path.split('/').last().unwrap_or("").strip_suffix(".conf").unwrap_or("");
            let mut ip = "Unknown".to_string();
            
            // Use doas to read the specific config
            if let Some(content) = utils::run_command_output(&format!("doas cat {}", path)) {
                for line in content.lines() {
                    if line.contains("Address = ") {
                        ip = line.split('=').nth(1).unwrap_or("").trim().split('/').next().unwrap_or("Unknown").to_string();
                    }
                }
            }
            println!("{:<15} {:<15} {:<15}", name, style("AVAILABLE").green(), ip);
        }
    } else {
        eprintln!("{} Could not read clients directory", style("[ERROR]").red());
    }
}

pub fn show_qr(name: &str) {
    // Uses 'doas cat' to read the restricted config file and pipes it to qrencode
    let cmd = format!("doas cat /etc/wireguard/clients/{}.conf | qrencode -t ansiutf8", name);
    
    if !utils::run_command(&cmd) {
        eprintln!("{} Failed to display QR code. Ensure 'qrencode' is installed.", 
            style("[ERROR]").red());
    }
}