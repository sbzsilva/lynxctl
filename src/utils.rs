use std::process::{Command, Stdio};

/// Check if a service is running by name (e.g., "unbound")
pub fn is_service_running(service_name: &str) -> bool {
    let result = Command::new("pgrep")
        .arg("-x")
        .arg(service_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
        
    match result {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Run a command and return its output as a string (useful for status checks)
pub fn run_command_output(cmd: &str) -> Option<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output(); // .output() captures stdout/stderr automatically
        
    match output {
        Ok(output) => {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                None
            }
        },
        Err(_) => None,
    }
}

/// Run a command silently and return if it was successful
/// Updated with Stdio::null() to prevent UI leakage
pub fn run_command(cmd: &str) -> bool {
    let result = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::null()) // Suppress raw command output
        .stderr(Stdio::null()) // Suppress error messages
        .status();
        
    match result {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}