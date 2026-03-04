pub fn get_active_peers_with_health() -> Vec<(String, String, String, u64)> {
    let mut peer_data = Vec::new();

    if let Some(output) = crate::utils::run_command_output("doas wg show wg0 dump") {
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 7 {
                let public_key = parts[0];
                let endpoint = parts[2].to_string();
                let handshake = parts[4].parse::<u64>().unwrap_or(0);
                let rx = parts[5].parse::<u64>().unwrap_or(0);
                let tx = parts[6].parse::<u64>().unwrap_or(0);

                // We use crate::WG_GW directly to satisfy the compiler
                let profile_cmd = format!(
                    "doas grep -l '{}' /etc/wireguard/clients/*.conf", 
                    public_key
                );
                
                let profile = crate::utils::run_command_output(&profile_cmd)
                    .map(|path| path.trim().split('/').last().unwrap_or("").replace(".conf", ""))
                    .unwrap_or_else(|| {
                        if public_key.len() > 10 {
                            format!("Key:{}..", &public_key[0..6])
                        } else {
                            "Unknown".to_string()
                        }
                    });

                let transfer_str = format!("{:.2}↑ / {:.2}↓ MB", tx as f32 / 1.0e6, rx as f32 / 1.0e6);
                peer_data.push((profile, endpoint, transfer_str, handshake));
            }
        }
    }
    peer_data
}