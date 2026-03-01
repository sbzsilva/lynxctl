pub fn get_active_peers_with_usage() -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut sessions = Vec::new();
    let mut usage = Vec::new();

    if let Some(output) = crate::utils::run_command_output("doas wg show wg0 dump") {
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 7 {
                let public_key = parts[0];
                let rx = parts[5].parse::<u64>().unwrap_or(0);
                let tx = parts[6].parse::<u64>().unwrap_or(0);

                let profile_cmd = format!("doas grep -l '{}' /etc/wireguard/clients/*.conf", public_key);
                let profile = crate::utils::run_command_output(&profile_cmd)
                    .map(|path| {
                        path.trim()
                            .split('/')
                            .last()
                            .unwrap_or("Unknown")
                            .replace(".conf", "")
                    })
                    .unwrap_or_else(|| "Profile Missing".to_string());

                let transfer_str = format!(
                    "{:.2} MB ↑ / {:.2} MB ↓",
                    tx as f32 / 1_000_000.0,
                    rx as f32 / 1_000_000.0
                );

                sessions.push((profile.trim().to_string(), "Active".to_string()));
                usage.push(("".to_string(), transfer_str));
            }
        }
    }
    (sessions, usage)
}