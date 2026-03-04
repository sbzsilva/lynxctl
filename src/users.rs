use anyhow::Result;
use console::style;
use crate::{utils, APP_ROOT};

pub fn create_user(name: &str) {
    match get_next_available_ip() {
        Ok(ip) => {
            let server_pub = utils::run_command_output("doas wg show wg0 public-key")
                .unwrap_or_default().trim().to_string();
            
            let server_ip = utils::run_command_output("curl -s ifconfig.me")
                .unwrap_or_default().trim().to_string();

            let cmd = format!(
                "priv=$(wg genkey); pub=$(echo $priv | wg pubkey); \
                template=\"# Profile: {}\n# PublicKey: $pub\n\n[Interface]\nPrivateKey = %s\nAddress = {}/32\nDNS = 10.200.200.1\n\n[Peer]\nPublicKey = {}\nEndpoint = {}:51820\nAllowedIPs = 0.0.0.0/0\n\"; \
                printf \"$template\" \"$priv\" | doas tee {}/etc/wireguard/clients/{}.conf > /dev/null",
                name, ip, server_pub, server_ip, APP_ROOT, name
            );
            
            if utils::run_command(&cmd) {
                println!("{} Profile for {} created.", style("[SUCCESS]").green(), name);
                show_existing_qr(name);
                crate::system::sync_kernel();
            }
        },
        Err(e) => eprintln!("{} No available IPs: {}", style("[ERROR]").red(), e),
    }
}

fn get_next_available_ip() -> Result<String> {
    for i in 2..255 {
        let target = format!("10.200.200.{}", i);
        let check_cmd = format!("doas grep -r \"{}\" {}/etc/wireguard/clients/", target, APP_ROOT);
        if !utils::run_command(&check_cmd) { return Ok(target); }
    }
    Err(anyhow::anyhow!("No available IPs"))
}

pub fn delete_user(name: &str) {
    let cmd = format!("doas rm {}/etc/wireguard/clients/{}.conf", APP_ROOT, name);
    if utils::run_command(&cmd) {
        println!("{} Deleted {}.conf", style("Deleted").red(), name);
        crate::system::sync_kernel(); 
    }
}

pub fn list_clients() {
    println!("\n {:<15} {:<15} {:<15}", style("Profile").bold(), style("Status").bold(), style("IP Address").bold());
    println!("{}", "─".repeat(50));

    let cmd = format!("doas ls {}/etc/wireguard/clients/*.conf 2>/dev/null", APP_ROOT);
    if let Some(output) = utils::run_command_output(&cmd) {
        for path in output.lines() {
            let name = path.split('/').last().unwrap_or("").replace(".conf", "");
            let mut ip = "Unknown".to_string();
            if let Some(content) = utils::run_command_output(&format!("doas cat {}", path)) {
                for line in content.lines() {
                    if line.trim().starts_with("Address") {
                        ip = line.split('=').nth(1).unwrap_or("").trim().split('/').next().unwrap_or("Unknown").to_string();
                    }
                }
            }
            println!("{:<15} {:<15} {:<15}", name, style("AVAILABLE").green(), ip);
        }
    }
}

pub fn show_existing_qr(name: &str) {
    let path = format!("{}/etc/wireguard/clients/{}.conf", APP_ROOT, name);
    println!("\nScan this code with the WireGuard App:");
    let cmd = format!("doas qrencode -t ansiutf8 < {}", path);
    utils::run_interactive_command(&cmd);
}