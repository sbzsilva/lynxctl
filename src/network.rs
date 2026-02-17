use console::style;

pub fn whitelist_domain(domain: &str) {
    // Write the domain to the whitelist config file
    let cmd = format!("echo 'local-zone: \"{}\" transparent' >> /var/unbound/etc/whitelist.conf", domain);
    
    if crate::utils::run_command(&cmd) {
        println!("{} Whitelisted {}. Restarting Unbound...", 
            style("[OK]").green(), domain);
        
        if crate::utils::run_command("rcctl restart unbound") {
            println!("Unbound restarted successfully");
        } else {
            eprintln!("Failed to restart Unbound");
        }
    } else {
        eprintln!("{} Failed to whitelist domain: {}", 
            style("[ERROR]").red(), domain);
    }
}

pub fn test_blocking() {
    println!("{} Testing DNS Shield...", style("[-]").cyan()); // Fixed styling call
    if let Some(result) = crate::utils::run_command_output("host doubleclick.net 127.0.0.1") {
        println!("{}", result);
    } else {
        eprintln!("DNS test command failed. Is 'host' installed?");
    }
}

// Export the functions from monitor module that were previously in network
pub use crate::monitor::{run_live_dashboard, show_status_dashboard};