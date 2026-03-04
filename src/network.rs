use console::style;
use crate::APP_ROOT;

pub fn whitelist_domain(domain: &str) {
    // UPDATED: Points to appliance whitelist path
    let cmd = format!("echo 'local-zone: \"{}\" transparent' >> {}/etc/unbound/whitelist.conf", domain, APP_ROOT);
    
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