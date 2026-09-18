mod task;
mod voicemeeter;

use std::env;

const DEFAULT_DELAY_MS: u64 = 75;

fn print_banner() {
    println!(
        r#"
  ___  __ _ _ __ _ __ | |_   _  __ _  __ _  ___ _ __ 
 / _ \/ _` | '__| '_ \| | | | |/ _` |/ _` |/ _ \ '__|
|  __/ (_| | |  | |_) | | |_| | (_| | (_| |  __/ |   
 \___|\__,_|_|  | .__/|_|\__,_|\__, |\__, |\___|_|   
                |_|            |___/ |___/           
    Automated Voicemeeter Auto-Restarter for KVM & USB
"#
    );
}

fn print_help() {
    print_banner();
    println!(
        r#"Usage: earplugger <COMMAND> [OPTIONS]

Commands:
  restart              Settle USB audio device and restart Voicemeeter engine (default)
  install              Register Windows Task Scheduler event trigger
  uninstall            Remove Windows Task Scheduler event trigger
  status               Check status of task trigger and Voicemeeter engine
  version              Print version information (--version, -v)
  help                 Print this message

Options for 'restart':
  --delay-ms <MS>      Millisecond delay to wait for USB handshake (default: 75)

Options for 'install':
  --device <NAME>      Device name filter (e.g. "RODE NT-USB"). If omitted, auto-detects A1.
  --delay-ms <MS>      Millisecond delay to configure in the trigger (default: 75)

Examples:
  earplugger restart
  earplugger install --device "RODE NT-USB"
  earplugger install
  earplugger status
  earplugger uninstall
"#
    );
}

fn parse_delay_arg(args: &[String], default: u64) -> Result<u64, String> {
    let mut delay = default;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--delay-ms" {
            if i + 1 >= args.len() {
                return Err("Missing value for --delay-ms".to_string());
            }
            delay = args[i + 1]
                .parse::<u64>()
                .map_err(|_| format!("Invalid integer value '{}' for --delay-ms", args[i + 1]))?;
            i += 2;
            continue;
        }
        i += 1;
    }
    Ok(delay)
}

fn parse_device_arg(args: &[String]) -> Result<Option<String>, String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--device" {
            if i + 1 >= args.len() {
                return Err("Missing value for --device".to_string());
            }
            return Ok(Some(args[i + 1].clone()));
        }
        i += 1;
    }
    Ok(None)
}

fn handle_restart(args: &[String]) {
    let delay_ms = match parse_delay_arg(args, DEFAULT_DELAY_MS) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[earplugger] Error: {}", e);
            std::process::exit(1);
        }
    };

    if !voicemeeter::is_voicemeeter_running() {
        // Silent exit if Voicemeeter isn't running (avoids background spam)
        return;
    }

    if let Err(e) = voicemeeter::restart_audio_engine(delay_ms) {
        eprintln!("[earplugger] Error: {}", e);
        std::process::exit(1);
    }
}

fn handle_install(args: &[String]) {
    print_banner();
    let delay_ms = match parse_delay_arg(args, DEFAULT_DELAY_MS) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut device_name = match parse_device_arg(args) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] Error: {}", e);
            std::process::exit(1);
        }
    };

    // If device not specified, try to auto-detect from active Voicemeeter A1
    if device_name.is_none() {
        if let Ok(a1) = voicemeeter::get_a1_device_name() {
            if !a1.is_empty() && a1 != "-" {
                println!("[*] Auto-detected Voicemeeter A1 device: {}", a1);
                // Extract clean hardware name if it contains e.g. "Speakers (RODE NT-USB)"
                let cleaned = if let Some(start) = a1.find('(') {
                    if let Some(end) = a1[start..].find(')') {
                        let inner = &a1[start + 1..start + end];
                        // Strip leading digits like "2- RODE NT-USB"
                        if let Some(dash) = inner.find("- ") {
                            inner[dash + 2..].to_string()
                        } else {
                            inner.to_string()
                        }
                    } else {
                        a1.clone()
                    }
                } else {
                    a1.clone()
                };

                println!("[*] Filtering trigger on device: '{}'", cleaned);
                device_name = Some(cleaned);
            }
        }
    }

    let dev_str = device_name.as_deref();
    println!(
        "[*] Registering Task Scheduler trigger '{}'...",
        task::TASK_NAME
    );
    match task::install_task(dev_str, delay_ms) {
        Ok(_) => {
            println!("[+] Successfully installed Task Scheduler event trigger!");
            println!("    Event Log : Microsoft-Windows-Audio/Operational");
            println!("    Event ID  : 65 (Audio device state changed to ACTIVE)");
            if let Some(dev) = dev_str {
                println!("    Filter    : DeviceName = '{}'", dev);
            } else {
                println!("    Filter    : Any playback device");
            }
            println!("    Delay     : {}ms settle time", delay_ms);
            println!(
                "\nEarplugger is now active. Switching your KVM will automatically resync Voicemeeter."
            );
        }
        Err(e) => {
            if e.to_lowercase().contains("access is denied") {
                eprintln!("[-] Installation failed: Administrator privileges are required.");
                eprintln!("    Please run 'earplugger install' from an elevated terminal (Run as Administrator).");
            } else {
                eprintln!("[-] Installation failed: {}", e);
            }
            std::process::exit(1);
        }
    }
}

fn handle_uninstall() {
    print_banner();
    println!(
        "[*] Removing Task Scheduler trigger '{}'...",
        task::TASK_NAME
    );
    match task::uninstall_task() {
        Ok(_) => {
            println!("[+] Successfully removed Task Scheduler event trigger.");
        }
        Err(e) => {
            if e.to_lowercase().contains("access is denied") {
                eprintln!("[-] Uninstall failed: Administrator privileges are required.");
                eprintln!("    Please run 'earplugger uninstall' from an elevated terminal (Run as Administrator).");
            } else {
                eprintln!("[-] Uninstall failed or task was not found: {}", e);
            }
        }
    }
}

fn handle_status() {
    print_banner();
    println!("=== Voicemeeter Status ===");
    let vm_running = voicemeeter::is_voicemeeter_running();
    println!(
        "  Process running : {}",
        if vm_running { "YES" } else { "NO" }
    );
    if let Some(dll) = voicemeeter::find_voicemeeter_dll() {
        println!("  Remote API DLL  : {}", dll.display());
    } else {
        println!("  Remote API DLL  : NOT FOUND");
    }

    if vm_running {
        if let Ok(a1) = voicemeeter::get_a1_device_name() {
            println!(
                "  Hardware A1     : {}",
                if a1.is_empty() { "None" } else { &a1 }
            );
        }
    }

    println!("\n=== Task Scheduler Trigger Status ===");
    match task::query_task_status() {
        Ok(info) => {
            println!("  Task status     : REGISTERED & READY");
            for line in info.lines().take(6) {
                println!("    {}", line);
            }
        }
        Err(_) => {
            println!("  Task status     : NOT REGISTERED");
            println!("  Run 'earplugger install' to activate.");
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        // Default execution (e.g. from Task Scheduler event trigger): run restart
        handle_restart(&[]);
        return;
    }

    match args[0].as_str() {
        "restart" => handle_restart(&args[1..]),
        "install" => handle_install(&args[1..]),
        "uninstall" => handle_uninstall(),
        "status" => handle_status(),
        "version" | "--version" | "-v" => {
            println!("earplugger {}", env!("CARGO_PKG_VERSION"));
        }
        "help" | "--help" | "-h" => print_help(),
        other => {
            eprintln!(
                "Unknown command: '{}'. Run 'earplugger help' for usage.",
                other
            );
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delay_arg_default() {
        let args: Vec<String> = vec![];
        let delay = parse_delay_arg(&args, 75).unwrap();
        assert_eq!(delay, 75);
    }

    #[test]
    fn test_parse_delay_arg_custom() {
        let args = vec!["--delay-ms".to_string(), "120".to_string()];
        let delay = parse_delay_arg(&args, 75).unwrap();
        assert_eq!(delay, 120);
    }

    #[test]
    fn test_parse_delay_arg_invalid() {
        let args = vec!["--delay-ms".to_string(), "invalid".to_string()];
        assert!(parse_delay_arg(&args, 75).is_err());
    }

    #[test]
    fn test_parse_device_arg() {
        let args = vec!["--device".to_string(), "RODE NT-USB".to_string()];
        let dev = parse_device_arg(&args).unwrap();
        assert_eq!(dev, Some("RODE NT-USB".to_string()));
    }
}
