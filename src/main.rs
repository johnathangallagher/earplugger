mod task;
mod voicemeeter;

use std::env;
use std::ffi::c_void;
use task::DEFAULT_DELAY_MS;

const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ELEVATION_CLASS: u32 = 20;

#[repr(C)]
struct TokenElevation {
    token_is_elevated: u32,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(
        process_handle: *mut c_void,
        desired_access: u32,
        token_handle: *mut *mut c_void,
    ) -> i32;
    fn GetTokenInformation(
        token_handle: *mut c_void,
        token_information_class: u32,
        token_information: *mut c_void,
        token_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

pub fn is_user_admin() -> bool {
    unsafe {
        let mut token: *mut c_void = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TokenElevation {
            token_is_elevated: 0,
        };
        let mut ret_len = 0u32;
        let success = GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut elevation as *mut _ as *mut c_void,
            std::mem::size_of::<TokenElevation>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);
        success != 0 && elevation.token_is_elevated != 0
    }
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
  version              Print version information (--version, -v, -V)
  help                 Print this message

Options for 'restart':
  --delay-ms <MS>      Millisecond delay to wait for USB handshake (default: 150, max: 30000)
  --silent             Suppress interactive output (used by Task Scheduler)

Options for 'install':
  --device <NAME>      Device name filter (e.g. "RODE NT-USB"). If omitted, auto-detects A1.
  --delay-ms <MS>      Millisecond delay to configure in the trigger (default: 150, max: 30000)
  --user <USERNAME>    Target user for scheduled task (e.g. DOMAIN\User)

Options for 'uninstall':
  --disable-channel    Also disable the Microsoft-Windows-Audio/Operational event channel

Examples:
  earplugger restart
  earplugger install --device "RODE NT-USB"
  earplugger install
  earplugger status
  earplugger uninstall --disable-channel
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

    // Windows endpoint friendly names across all languages are formatted as:
    // "<Endpoint Role> (<Hardware Description>)"
    // e.g., "Speakers (Realtek(R) Audio)", "Lautsprecher (RODE NT-USB)", "Altavoces (USB Audio)"
    // The hardware description is enclosed in the outermost parenthetical group ending at the end of the string.
    if s.ends_with(')') {
        if let Some(first_paren) = s.find(" (") {
            let inner = s[first_paren + 2..s.len() - 1].trim();
            if inner.chars().any(|c| c.is_alphabetic()) && inner.len() > 1 {
                s = inner;
            }
        }
    }

    // Strip leading Windows device endpoint indices like "2- RODE NT-USB"
    if let Some(dash_idx) = s.find("- ") {
        let prefix = &s[..dash_idx];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            s = s[dash_idx + 2..].trim();
        }
    }

    s.to_string()
}

pub struct ParsedArgs {
    pub delay_ms: u64,
    pub device: Option<String>,
    pub user: Option<String>,
    pub silent: bool,
    pub disable_channel: bool,
    pub help_requested: bool,
}

pub fn parse_options(args: &[String]) -> Result<ParsedArgs, String> {
    let mut delay_ms = DEFAULT_DELAY_MS;
    let mut device: Option<String> = None;
    let mut user: Option<String> = None;
    let mut silent = false;
    let mut disable_channel = false;
    let mut help_requested = false;
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "--help" || arg == "-h" {
            help_requested = true;
            i += 1;
        } else if arg == "--silent" {
            silent = true;
            i += 1;
        } else if arg == "--disable-channel" {
            disable_channel = true;
            i += 1;
        } else if arg == "--delay-ms" {
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
            let val = args[i + 1].trim();
            if val.is_empty() {
                return Err("Value for --device cannot be empty".to_string());
            }
            device = Some(val.to_string());
            i += 2;
        } else if let Some(val_str) = arg.strip_prefix("--device=") {
            let val = val_str.trim();
            if val.is_empty() {
                return Err("Value for --device cannot be empty".to_string());
            }
            device = Some(val.to_string());
            i += 1;
        } else if arg == "--user" {
            if i + 1 >= args.len() {
                return Err("Missing value for --user".to_string());
            }
            let val = args[i + 1].trim();
            if val.is_empty() {
                return Err("Value for --user cannot be empty".to_string());
            }
            user = Some(val.to_string());
            i += 2;
        } else if let Some(val_str) = arg.strip_prefix("--user=") {
            let val = val_str.trim();
            if val.is_empty() {
                return Err("Value for --user cannot be empty".to_string());
            }
            user = Some(val.to_string());
            i += 1;
        } else {
            return Err(format!("Unrecognized option: '{}'", arg));
        }
    }

    Ok(ParsedArgs {
        delay_ms,
        device,
        user,
        silent,
        disable_channel,
        help_requested,
    })
}

fn handle_restart(args: &[String]) {
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[earplugger] Error: {}", e);
            std::process::exit(1);
        }
    };

    if opts.help_requested {
        print_help();
        return;
    }

    if opts.device.is_some() || opts.user.is_some() || opts.disable_channel {
        eprintln!(
            "[earplugger] Error: Invalid options for 'restart'. Only --delay-ms and --silent are accepted."
        );
        std::process::exit(1);
    }

    // Verify Voicemeeter is running BEFORE blocking on delay_ms
    if !voicemeeter::is_voicemeeter_running() {
        if !opts.silent {
            eprintln!(
                "[earplugger] Error: Voicemeeter process not detected. Audio engine restart skipped."
            );
            std::process::exit(1);
        }
        return;
    }

    if opts.delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(opts.delay_ms));
    }

    if let Err(e) = voicemeeter::restart_audio_engine(0) {
        if !opts.silent {
            eprintln!("[earplugger] Error: {}", e);
            std::process::exit(1);
        } else {
            // In silent mode, exit cleanly if Voicemeeter was closed during the delay window
            if e.contains("not running") {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
    } else if !opts.silent {
        println!("[earplugger] Successfully triggered Voicemeeter audio engine restart.");
    }
}

fn handle_install(args: &[String]) {
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[-] Error: {}", e);
            std::process::exit(1);
        }
    };

    if opts.help_requested {
        print_help();
        return;
    }

    if opts.silent || opts.disable_channel {
        eprintln!(
            "[-] Error: Invalid options for 'install'. Only --device, --delay-ms, and --user are accepted."
        );
        std::process::exit(1);
    }

    print_banner();

    if !is_user_admin() {
        eprintln!("[-] Administrator privileges are required to install scheduled tasks.");
        eprintln!(
            "    Please run 'earplugger install' from an elevated terminal (Run as Administrator)."
        );
        std::process::exit(1);
    }

    let mut device_name = opts.device;

    // If device not specified, try to auto-detect from active Voicemeeter A1
    if device_name.is_none() {
        match voicemeeter::get_a1_device_name() {
            Ok(a1) if !a1.is_empty() && a1 != "-" => {
                println!("[*] Auto-detected Voicemeeter A1 device: {}", a1);
                let cleaned = clean_device_name(&a1);
                println!("[*] Filtering trigger on device: '{}'", cleaned);
                device_name = Some(cleaned);
            }
            _ => {
                println!(
                    "[!] Warning: Voicemeeter is not running or no active Hardware A1 device was detected."
                );
                println!("    Installing wildcard event trigger for any active audio endpoint.");
            }
        }
    }

    let dev_str = device_name.as_deref();
    println!(
        "[*] Registering Task Scheduler trigger '{}'...",
        task::TASK_NAME
    );
    match task::install_task(dev_str, opts.delay_ms, opts.user.as_deref()) {
        Ok(_) => {
            println!("[+] Successfully installed Task Scheduler event trigger!");
            println!("    Event Log : Microsoft-Windows-Audio/Operational");
            println!("    Event ID  : 65 (Audio device state changed to ACTIVE)");
            if let Some(dev) = dev_str {
                println!("    Filter    : DeviceName = '{}'", dev);
            } else {
                println!("    Filter    : Any active audio endpoint");
            }
            if let Some(ref u) = opts.user {
                println!("    User      : {}", u);
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

fn handle_uninstall(args: &[String]) {
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[-] Error: {}", e);
            std::process::exit(1);
        }
    };

    if opts.help_requested {
        print_help();
        return;
    }

    if opts.device.is_some()
        || opts.user.is_some()
        || opts.silent
        || opts.delay_ms != DEFAULT_DELAY_MS
    {
        eprintln!(
            "[-] Error: Invalid options for 'uninstall'. Only --disable-channel is accepted."
        );
        std::process::exit(1);
    }

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
    match task::uninstall_task(opts.disable_channel) {
        Ok(_) => {
            println!("[+] Successfully removed Task Scheduler event trigger.");
            if opts.disable_channel {
                println!("[+] Disabled Microsoft-Windows-Audio/Operational event channel.");
            } else {
                println!(
                    "    Note: The Microsoft-Windows-Audio/Operational event channel remains enabled."
                );
                println!(
                    "          Run 'earplugger uninstall --disable-channel' if you wish to disable it."
                );
            }
        }
        Err(e) => {
            eprintln!("[-] Uninstall failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_status(args: &[String]) {
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[-] Error: {}", e);
            std::process::exit(1);
        }
    };

    if opts.help_requested {
        print_help();
        return;
    }

    if opts.device.is_some()
        || opts.user.is_some()
        || opts.silent
        || opts.disable_channel
        || opts.delay_ms != DEFAULT_DELAY_MS
    {
        eprintln!("[-] Error: 'status' takes no extra options.");
        std::process::exit(1);
    }

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
        Ok(status) => {
            let status_desc = if !status.is_enabled {
                "REGISTERED (DISABLED)"
            } else {
                "REGISTERED & READY"
            };
            println!("  Task status     : {}", status_desc);
            println!("  XML definition  : Valid ({} bytes)", status.xml_raw.len());
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
        handle_restart(&[]);
        return;
    }

    match args[0].as_str() {
        "restart" => handle_restart(&args[1..]),
        "install" => handle_install(&args[1..]),
        "uninstall" => handle_uninstall(&args[1..]),
        "status" => handle_status(&args[1..]),
        "version" | "--version" | "-v" | "-V" => {
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
        assert!(!opts.silent);
        assert!(!opts.help_requested);
    }

    #[test]
    fn test_parse_options_custom_space() {
        let args = vec![
            "--delay-ms".to_string(),
            "120".to_string(),
            "--device".to_string(),
            "RODE NT-USB".to_string(),
            "--silent".to_string(),
        ];
        let opts = parse_options(&args).unwrap();
        assert_eq!(opts.delay_ms, 120);
        assert_eq!(opts.device, Some("RODE NT-USB".to_string()));
        assert!(opts.silent);
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
    fn test_parse_options_help_flag() {
        let args = vec!["--help".to_string()];
        let opts = parse_options(&args).unwrap();
        assert!(opts.help_requested);
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
    fn test_clean_device_name_trademark_preserved() {
        // Crucial test: Realtek(R) Audio and Intel(R) Display Audio must NOT be mangled to "R"
        assert_eq!(clean_device_name("Realtek(R) Audio"), "Realtek(R) Audio");
        assert_eq!(
            clean_device_name("Intel(R) Display Audio"),
            "Intel(R) Display Audio"
        );
    }

    #[test]
    fn test_clean_device_name_no_parentheses() {
        assert_eq!(clean_device_name("DirectSound: Audio Out"), "Audio Out");
    }

    #[test]
    fn test_clean_device_name_multilingual() {
        // German
        assert_eq!(
            clean_device_name("WDM: Lautsprecher (Realtek(R) Audio)"),
            "Realtek(R) Audio"
        );
        assert_eq!(
            clean_device_name("MME: Kopfhörer (RODE NT-USB)"),
            "RODE NT-USB"
        );
        // French
        assert_eq!(
            clean_device_name("WDM: Haut-parleurs (2- Focusrite USB)"),
            "Focusrite USB"
        );
        // Spanish
        assert_eq!(
            clean_device_name("WDM: Altavoces (High Definition Audio Device)"),
            "High Definition Audio Device"
        );
        // Japanese
        assert_eq!(
            clean_device_name("WDM: スピーカー (USB Audio CODEC)"),
            "USB Audio CODEC"
        );
        // Chinese
        assert_eq!(
            clean_device_name("WDM: 扬声器 (Realtek(R) Audio)"),
            "Realtek(R) Audio"
        );
    }

    #[test]
    fn test_parse_options_device_empty_errors() {
        let args_space = vec!["--device".to_string(), "".to_string()];
        assert!(parse_options(&args_space).is_err());

        let args_equals = vec!["--device=".to_string()];
        assert!(parse_options(&args_equals).is_err());
    }

    #[test]
    fn test_parse_options_user_and_disable_channel() {
        let args = vec![
            "--user".to_string(),
            "WORKGROUP\\User1".to_string(),
            "--disable-channel".to_string(),
        ];
        let opts = parse_options(&args).unwrap();
        assert_eq!(opts.user, Some("WORKGROUP\\User1".to_string()));
        assert!(opts.disable_channel);
    }
}
