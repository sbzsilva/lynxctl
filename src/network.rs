use console::style;

pub fn whitelist_domain(domain: &str) {
    // Write the domain to the whitelist config file
    let cmd = format!("echo 'local-zone: \"{}\" transparent' >> /var/unbound/etc/whitelist.conf", domain);
    
    if crate::utils::run_command(&cmd) {
        println!("{} Whitelisted {}. Restarting Unbound...", 
            style("[OK]").green(), domain);
        
        if crate::utils::run_command("doas rcctl restart unbound") {
            println!("Unbound restarted successfully");
        } else {
            eprintln!("Failed to restart Unbound");
        }
    } else {
        eprintln!("{} Failed to whitelist domain: {}", 
            style("[ERROR]").red(), domain);
    }
}