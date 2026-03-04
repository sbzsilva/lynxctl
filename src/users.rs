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

            // Atomic profile generation
            let cmd = format!(
                "priv=$(wg genkey); pub=$(echo $priv | wg pubkey); \
                template=\"# Profile: {}\n# PublicKey: $pub\n\n[Interface]\nPrivateKey = %s\nAddress = {}/32\nDNS = 10.200.200.1\n\n[Peer]\nPublicKey = {}\nEndpoint = {}:51820\nAllowedIPs = 0.0.0.0/0\n\"; \
                printf \"$template\" \"$priv\" | doas tee {}/etc/wireguard/clients/{}.conf > /dev/null",
                name, ip, server_pub, server_ip, APP_ROOT, name
            );
            
            if utils::run_command(&cmd) {
                println!("{} Profile for {} created at {}/etc/wireguard/clients/", 
                    style("[SUCCESS]").green(), name, APP_ROOT);
                show_existing_qr(name);
            }
        },
        Err(e) => eprintln!("{} IP Allocation Failed: {}", style("[ERROR]").red(), e),
    }
}

fn get_next_available_ip() -> Result<String> {
    for i in 2..255 {
        let target = format!("10.200.200.{}", i);
        let check_cmd = format!("grep -r \"{}\" {}/etc/wireguard/clients/", target, APP_ROOT);
        if !utils::run_command(&check_cmd) { return Ok(target); }
    }
    Err(anyhow::anyhow!("IP Pool Exhausted"))
}

pub fn show_existing_qr(name: &str) {
    let path = format!("{}/etc/wireguard/clients/{}.conf", APP_ROOT, name);
    println!("\nScan this code for {}:", style(name).cyan());
    // Use ansiutf8 for high-fidelity terminal display
    let cmd = format!("doas qrencode -t ansiutf8 < {}", path);
    utils::run_interactive_command(&cmd);
}

pub fn list_clients() {
    let cmd = format!("ls {}/etc/wireguard/clients/*.conf 2>/dev/null", APP_ROOT);
    if let Some(output) = utils::run_command_output(&cmd) {
        for line in output.lines() {
            let name = line.split('/').last().unwrap_or("").replace(".conf", "");
            println!(" - {}", style(name).green());
        }
    }
}

pub fn delete_user(name: &str) {
    let cmd = format!("doas rm {}/etc/wireguard/clients/{}.conf", APP_ROOT, name);
    if utils::run_command(&cmd) {
        println!("{} Profile {} removed.", style("[DELETED]").red(), name);
        crate::system::sync_kernel();
    }
}