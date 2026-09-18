mod task;
mod voicemeeter;

use std::env;
use task::DEFAULT_DELAY_MS;

#[link(name = "shell32")]
unsafe extern "system" {
    fn IsUserAnAdmin() -> i32;
}

pub fn is_user_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

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
  --delay-ms <MS>      Millisecond delay to wait for USB handshake (default: 150, max: 30000)

Options for 'install':
  --device <NAME>      Device name filter (e.g. "RODE NT-USB"). If omitted, auto-detects A1.
  --delay-ms <MS>      Millisecond delay to configure in the trigger (default: 150, max: 30000)

Examples:
  earplugger restart
  earplugger install --device "RODE NT-USB"
  earplugger install
  earplugger status
  earplugger uninstall
"#
    );
}

pub fn clean_device_name(raw: &str) -> String {
    let mut s = raw.trim();

    // Strip driver type prefix if present (e.g. "WDM: ", "MME: ", "KS: ", "ASIO: ", "DirectSound: ")
    if let Some(colon_idx) = s.find(": ") {
        let prefix = &s[..colon_idx];
        if !prefix.is_empty() && prefix.chars().all(|c| c.is_alphanumeric() || c == '_') {
            s = s[colon_idx + 2..].trim();
        }
    }

    // Extract device from outer parentheses: e.g. "Speakers (Realtek(R) Audio)" -> "Realtek(R) Audio"
    let candidate = if let Some(start) = s.find('(') {
        if let Some(end) = s.rfind(')') {
            if end > start { &s[start + 1..end] } else { s }
        } else {
            s
        }
    } else {
        s
    };

    let trimmed = candidate.trim();

    // Strip leading Windows device endpoint indices like "2- RODE NT-USB"
    if let Some(dash_idx) = trimmed.find("- ") {
        let prefix = &trimmed[..dash_idx];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            return trimmed[dash_idx + 2..].trim().to_string();
        }
    }

    trimmed.to_string()
}

pub struct ParsedArgs {
    pub delay_ms: u64,
    pub device: Option<String>,
}

pub fn parse_options(args: &[String]) -> Result<ParsedArgs, String> {
    let mut delay_ms = DEFAULT_DELAY_MS;
    let mut device: Option<String> = None;
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "--delay-ms" {
            if i + 1 >= args.len() {
                return Err("Missing value for --delay-ms".to_string());
            }
            let val = args[i + 1]
                .parse::<u64>()
                .map_err(|_| format!("Invalid integer value '{}' for --delay-ms", args[i + 1]))?;
            if val > 30000 {
                return Err(format!(
                    "Delay value {}ms exceeds maximum allowed threshold of 30000ms",
                    val
                ));
            }
            delay_ms = val;
            i += 2;
        } else if let Some(val_str) = arg.strip_prefix("--delay-ms=") {
            let val = val_str
                .parse::<u64>()
                .map_err(|_| format!("Invalid integer value '{}' for --delay-ms", val_str))?;
            if val > 30000 {
                return Err(format!(
                    "Delay value {}ms exceeds maximum allowed threshold of 30000ms",
                    val
                ));
            }
            delay_ms = val;
            i += 1;
        } else if arg == "--device" {
            if i + 1 >= args.len() {
                return Err("Missing value for --device".to_string());
            }
            device = Some(args[i + 1].clone());
            i += 2;
        } else if let Some(val_str) = arg.strip_prefix("--device=") {
            device = Some(val_str.to_string());
            i += 1;
        } else {
            return Err(format!("Unrecognized option: '{}'", arg));
        }
    }

    Ok(ParsedArgs { delay_ms, device })
}

fn handle_restart(args: &[String], is_interactive: bool) {
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[earplugger] Error: {}", e);
            std::process::exit(1);
        }
    };

    if opts.delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(opts.delay_ms));
    }

    if !voicemeeter::is_voicemeeter_running() {
        if is_interactive {
            println!(
                "[earplugger] Voicemeeter process not detected. Audio engine restart skipped."
            );
        }
        return;
    }

    if let Err(e) = voicemeeter::restart_audio_engine(0) {
        eprintln!("[earplugger] Error: {}", e);
        std::process::exit(1);
    } else if is_interactive {
        println!("[earplugger] Successfully triggered Voicemeeter audio engine restart.");
    }
}

fn handle_install(args: &[String]) {
    print_banner();

    if !is_user_admin() {
        eprintln!("[-] Administrator privileges are required to install scheduled tasks.");
        eprintln!(
            "    Please run 'earplugger install' from an elevated terminal (Run as Administrator)."
        );
        std::process::exit(1);
    }

    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[-] Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut device_name = opts.device;

    // If device not specified, try to auto-detect from active Voicemeeter A1
    if device_name.is_none() {
        let a1 = voicemeeter::get_a1_device_name().unwrap_or_default();
        if !a1.is_empty() && a1 != "-" {
            println!("[*] Auto-detected Voicemeeter A1 device: {}", a1);
            let cleaned = clean_device_name(&a1);
            println!("[*] Filtering trigger on device: '{}'", cleaned);
            device_name = Some(cleaned);
        }
    }

    let dev_str = device_name.as_deref();
    println!(
        "[*] Registering Task Scheduler trigger '{}'...",
        task::TASK_NAME
    );
    match task::install_task(dev_str, opts.delay_ms) {
        Ok(_) => {
            println!("[+] Successfully installed Task Scheduler event trigger!");
            println!("    Event Log : Microsoft-Windows-Audio/Operational");
            println!("    Event ID  : 65 (Audio device state changed to ACTIVE)");
            if let Some(dev) = dev_str {
                println!("    Filter    : DeviceName = '{}'", dev);
            } else {
                println!("    Filter    : Any active audio endpoint");
            }
            println!("    Delay     : {}ms settle time", opts.delay_ms);
            println!(
                "\nEarplugger is now active. Switching your KVM will automatically resync Voicemeeter."
            );
        }
        Err(e) => {
            eprintln!("[-] Installation failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_uninstall() {
    print_banner();

    if !is_user_admin() {
        eprintln!("[-] Administrator privileges are required to remove scheduled tasks.");
        eprintln!(
            "    Please run 'earplugger uninstall' from an elevated terminal (Run as Administrator)."
        );
        std::process::exit(1);
    }

    println!(
        "[*] Removing Task Scheduler trigger '{}'...",
        task::TASK_NAME
    );
    match task::uninstall_task() {
        Ok(_) => {
            println!("[+] Successfully removed Task Scheduler event trigger.");
        }
        Err(e) => {
            eprintln!("[-] Uninstall failed: {}", e);
            std::process::exit(1);
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
        let a1 = voicemeeter::get_a1_device_name().unwrap_or_default();
        let display = if a1.is_empty() || a1 == "-" {
            "None"
        } else {
            &a1
        };
        println!("  Hardware A1     : {}", display);
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
        // Default execution (e.g. from Task Scheduler event trigger): run restart non-interactively
        handle_restart(&[], false);
        return;
    }

    match args[0].as_str() {
        "restart" => handle_restart(&args[1..], true),
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
    fn test_parse_options_defaults() {
        let args: Vec<String> = vec![];
        let opts = parse_options(&args).unwrap();
        assert_eq!(opts.delay_ms, DEFAULT_DELAY_MS);
        assert_eq!(opts.device, None);
    }

    #[test]
    fn test_parse_options_custom_space() {
        let args = vec![
            "--delay-ms".to_string(),
            "120".to_string(),
            "--device".to_string(),
            "RODE NT-USB".to_string(),
        ];
        let opts = parse_options(&args).unwrap();
        assert_eq!(opts.delay_ms, 120);
        assert_eq!(opts.device, Some("RODE NT-USB".to_string()));
    }

    #[test]
    fn test_parse_options_custom_equals() {
        let args = vec![
            "--delay-ms=250".to_string(),
            "--device=User's AirPods".to_string(),
        ];
        let opts = parse_options(&args).unwrap();
        assert_eq!(opts.delay_ms, 250);
        assert_eq!(opts.device, Some("User's AirPods".to_string()));
    }

    #[test]
    fn test_parse_options_delay_bounds() {
        let args = vec!["--delay-ms=35000".to_string()];
        assert!(parse_options(&args).is_err());
    }

    #[test]
    fn test_parse_options_unknown_flag() {
        let args = vec!["--unknown-flag".to_string()];
        assert!(parse_options(&args).is_err());
    }

    #[test]
    fn test_clean_device_name_driver_prefixes() {
        assert_eq!(
            clean_device_name("WDM: Speakers (Realtek(R) Audio)"),
            "Realtek(R) Audio"
        );
        assert_eq!(
            clean_device_name("MME: Headset Earphone (2- RODE NT-USB)"),
            "RODE NT-USB"
        );
        assert_eq!(
            clean_device_name("KS: Headphones (High Definition Audio Device)"),
            "High Definition Audio Device"
        );
        assert_eq!(
            clean_device_name("ASIO: Focusrite USB ASIO"),
            "Focusrite USB ASIO"
        );
    }

    #[test]
    fn test_clean_device_name_no_parentheses() {
        assert_eq!(clean_device_name("DirectSound: Audio Out"), "Audio Out");
    }
}
