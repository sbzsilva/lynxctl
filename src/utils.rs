/// Run an interactive command that may need to prompt for passwords
pub fn run_interactive_command(cmd: &str) -> bool {
    // Enrich the PATH to ensure tools like qrencode are found
    let enriched_cmd = format!("export PATH=\"$PATH:/usr/local/bin:/usr/bin:/bin\"; {}", cmd);
    let result = Command::new("sh")
        .arg("-c")
        .arg(&enriched_cmd)
        .status();
        
    match result {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("Failed to execute interactive command: {}", e);
            false
        },
    }
}
use std::process::{Command, Stdio};

/// Check if a service is running by name (e.g., "unbound")
pub fn is_service_running(service_name: &str) -> bool {
    let enriched_cmd = format!("export PATH=\"$PATH:/usr/local/bin:/usr/bin:/bin\"; pgrep -x {}", service_name);
    let result = Command::new("sh")
        .arg("-c")
        .arg(enriched_cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
        
    match result {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Run a command and return its output as a string (Silent for TUI)
pub fn run_command_output(cmd: &str) -> Option<String> {
    let enriched_cmd = format!("export PATH=\"$PATH:/usr/local/bin:/usr/bin:/bin\"; {}", cmd);
    let output = Command::new("sh")
        .arg("-c")
        .arg(&enriched_cmd)
        .output(); // .output() captures both stdout and stderr
        
    match output {
        Ok(out) => {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).to_string())
            } else {
                // Return None silently to avoid corrupting the TUI display
                None
            }
        },
        Err(_) => None,
    }
}

/// Run a command silently and return if it was successful
pub fn run_command(cmd: &str) -> bool {
    // Enrich the PATH to ensure tools like qrencode are found
    let enriched_cmd = format!("export PATH=\"$PATH:/usr/local/bin:/usr/bin:/bin\"; {}", cmd);
    let result = Command::new("sh")
        .arg("-c")
        .arg(enriched_cmd)
        .stdout(Stdio::null()) // Suppress raw command output
        .stderr(Stdio::null()) // Suppress error messages
        .status();
        
    match result {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("Failed to execute command: {}", e);
            false
        },
    }
}