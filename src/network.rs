use console::style;
use crate::APP_ROOT;

pub fn whitelist_domain(domain: &str) {
    let cmd = format!("echo 'local-zone: \"{}\" transparent' >> {}/etc/unbound/whitelist.conf", domain, APP_ROOT);
    
    if crate::utils::run_command(&cmd) {
        println!("{} Whitelisted {}. Syncing Jail...", style("[OK]").green(), domain);
        // Sync master to jail
        crate::utils::run_command(&format!("doas cp {}/etc/unbound/whitelist.conf /var/unbound/etc/whitelist.conf", APP_ROOT));
        crate::utils::run_command("doas rcctl restart unbound");
    } else {
        eprintln!("{} Failed to whitelist domain: {}", style("[ERROR]").red(), domain);
    }
}