use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

pub const TASK_NAME: &str = "Earplugger_AutoRestart";
pub const DEFAULT_DELAY_MS: u64 = 150;

pub fn get_exe_path() -> Result<PathBuf, String> {
    env::current_exe().map_err(|e| format!("Failed to resolve current exe path: {}", e))
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemDirectoryW(lpBuffer: *mut u16, uSize: u32) -> u32;
    fn MultiByteToWideChar(
        CodePage: u32,
        dwFlags: u32,
        lpMultiByteStr: *const u8,
        cbMultiByte: i32,
        lpWideCharStr: *mut u16,
        cchWideChar: i32,
    ) -> i32;
}

pub fn decode_process_output(raw: &[u8]) -> String {
    if raw.is_empty() {
        return String::new();
    }
    // Check for UTF-16 LE BOM (0xFF, 0xFE) emitted by certain Windows commands
    if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
        let u16_data: Vec<u16> = raw[2..]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        return String::from_utf16_lossy(&u16_data).trim().to_string();
    }
    // If output is valid UTF-8, prefer it directly
    if let Ok(s) = std::str::from_utf8(raw) {
        return s.trim().to_string();
    }
    // Decode via Win32 OEM code page (CP_OEMCP = 1) for localized Windows console error messages
    let raw_len = i32::try_from(raw.len()).unwrap_or(i32::MAX);
    unsafe {
        const CP_OEMCP: u32 = 1;
        let len = MultiByteToWideChar(CP_OEMCP, 0, raw.as_ptr(), raw_len, std::ptr::null_mut(), 0);
        if len > 0 {
            let mut wide = vec![0u16; len as usize];
            let written =
                MultiByteToWideChar(CP_OEMCP, 0, raw.as_ptr(), raw_len, wide.as_mut_ptr(), len);
            if written > 0 {
                return String::from_utf16_lossy(&wide[..written as usize])
                    .trim()
                    .to_string();
            }
        }
    }
    String::from_utf8_lossy(raw).trim().to_string()
}

fn get_system32_path(binary: &str) -> PathBuf {
    // Initial attempt with a MAX_PATH buffer. If the path is longer (long-path-enabled systems),
    // retry with the exact required size returned by the API.
    let mut buf = vec![0u16; 260];
    let mut len = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) };

    if len as usize >= buf.len() {
        // Buffer was too small; len now holds the required character count including NUL.
        buf.resize(len as usize + 1, 0);
        len = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) };
    }

    let sys_dir = if len > 0 && (len as usize) < buf.len() {
        let s = String::from_utf16_lossy(&buf[..len as usize]);
        PathBuf::from(s)
    } else {
        // Final fallback: use the SystemRoot env var. Note that env vars are user-controlled,
        // but we only reach here on exotic long-path configurations where GetSystemDirectoryW
        // fails even after a retry — a case where no path is fully trustworthy.
        let sys_root = env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        PathBuf::from(sys_root).join("System32")
    };

    #[cfg(target_pointer_width = "32")]
    {
        // On a 64-bit OS running a 32-bit binary, System32 is redirected to SysWOW64.
        // Sysnative is a virtual alias that bypasses the redirector and reaches the real
        // 64-bit System32, where schtasks.exe and wevtutil.exe live.
        // On a native 32-bit OS, Sysnative does not exist and we fall through to sys_dir.
        if let Some(parent) = sys_dir.parent() {
            let sysnative = parent.join("Sysnative").join(binary);
            if sysnative.exists() {
                return sysnative;
            }
        }
    }

    sys_dir.join(binary)
}

pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        // Filter out illegal XML 1.0 control characters (anything < 0x20 except tab/LF/CR).
        // Device names with such characters are rejected at parse_options; this guard is
        // a defensive second layer.
        if c < ' ' && c != '\t' && c != '\n' && c != '\r' {
            continue;
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn format_xpath_string_literal(s: &str) -> String {
    // Strip control characters
    let clean: String = s.chars().filter(|&c| c >= ' ' || c == '\t').collect();
    if !clean.contains('\'') {
        format!("'{}'", clean)
    } else if !clean.contains('"') {
        // Windows Event Log query engine (wevtapi.dll) supports only a restricted subset of XPath 1.0;
        // functions like concat() are explicitly unsupported (error 15008 / ERROR_EVT_INVALID_QUERY).
        // For literals containing single quotes (e.g. "User's AirPods"), wrap in double quotes.
        // During XML emission, double quotes become &quot;, which Task Scheduler decodes back to
        // double quotes when creating the Event Log subscription.
        format!("\"{}\"", clean)
    } else {
        // If the string contains both single and double quotes, strip double quotes so the query
        // is valid without invoking unsupported XPath concat().
        let sanitized: String = clean.chars().filter(|&c| c != '"').collect();
        format!("\"{}\"", sanitized)
    }
}

pub fn generate_task_xml(
    exe_path: &str,
    device_filter: Option<&str>,
    delay_ms: u64,
    user: Option<&str>,
) -> String {
    let filter_clause = match device_filter {
        Some(dev) => {
            let formatted_literal = format_xpath_string_literal(dev);
            format!(
                " and *[EventData[Data[@Name='DeviceName']={} and (Data[@Name='flow']='0' or Data[@Name='flow']='1') and Data[@Name='NewState']='1']]",
                formatted_literal
            )
        }
        None => {
            " and *[EventData[(Data[@Name='flow']='0' or Data[@Name='flow']='1') and Data[@Name='NewState']='1']]".to_string()
        }
    };

    let subscription = format!(
        "<QueryList><Query Id=\"0\" Path=\"Microsoft-Windows-Audio/Operational\"><Select Path=\"Microsoft-Windows-Audio/Operational\">*[System[Provider[@Name='Microsoft-Windows-Audio'] and (EventID=65)]]{}</Select></Query></QueryList>",
        filter_clause
    );

    let escaped_subscription = xml_escape(&subscription);
    let escaped_exe = xml_escape(exe_path);

    let args_val = if delay_ms != DEFAULT_DELAY_MS {
        format!("restart --delay-ms {} --silent", delay_ms)
    } else {
        "restart --silent".to_string()
    };
    let args_elem = format!("<Arguments>{}</Arguments>", xml_escape(&args_val));

    let user_elem = match user {
        Some(u) => {
            let trimmed = u.trim();
            let escaped = xml_escape(trimmed);
            if !escaped.is_empty() {
                format!("\n      <UserId>{}</UserId>", escaped)
            } else {
                String::new()
            }
        }
        _ => String::new(),
    };

    // Scale ExecutionTimeLimit to accommodate the configured delay plus a 30-second buffer.
    // For default 150ms delay, this yields PT30S. For maximum 30,000ms delay, this yields PT60S,
    // preventing Task Scheduler from forcefully terminating the process before Voicemeeter
    // finishes restarting and drops FFI resources cleanly.
    let time_limit_secs = ((delay_ms / 1000) + 30).max(30);
    let time_limit_iso = format!("PT{}S", time_limit_secs);

    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Earplugger - Auto-restart Voicemeeter audio engine on USB/KVM reconnect</Description>
    <Author>Earplugger</Author>
  </RegistrationInfo>
  <Triggers>
    <EventTrigger>
      <Enabled>true</Enabled>
      <Subscription>{}</Subscription>
    </EventTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">{}
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>true</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>{}</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
      {}
    </Exec>
  </Actions>
</Task>"#,
        escaped_subscription, user_elem, time_limit_iso, escaped_exe, args_elem
    )
}

pub fn enable_audio_operational_log() -> Result<(), String> {
    let wevtutil = get_system32_path("wevtutil.exe");
    let output = Command::new(wevtutil)
        .args(["sl", "Microsoft-Windows-Audio/Operational", "/e:true"])
        .output()
        .map_err(|e| format!("Failed to invoke wevtutil.exe: {}", e))?;

    if !output.status.success() {
        let err = decode_process_output(&output.stderr);
        let out = decode_process_output(&output.stdout);
        let msg = format!("{} {}", out, err).trim().to_string();
        return Err(format!(
            "wevtutil failed to enable Microsoft-Windows-Audio/Operational log: {}",
            msg
        ));
    }

    Ok(())
}

pub fn disable_audio_operational_log() -> Result<(), String> {
    let wevtutil = get_system32_path("wevtutil.exe");
    let output = Command::new(wevtutil)
        .args(["sl", "Microsoft-Windows-Audio/Operational", "/e:false"])
        .output()
        .map_err(|e| format!("Failed to invoke wevtutil.exe: {}", e))?;

    if !output.status.success() {
        let err = decode_process_output(&output.stderr);
        let out = decode_process_output(&output.stdout);
        let msg = format!("{} {}", out, err).trim().to_string();
        return Err(format!(
            "wevtutil failed to disable Microsoft-Windows-Audio/Operational log: {}",
            msg
        ));
    }

    Ok(())
}

struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn install_task(
    device_filter: Option<&str>,
    delay_ms: u64,
    user: Option<&str>,
) -> Result<(), String> {
    // Ensure the Windows Audio Operational event channel is active
    enable_audio_operational_log()?;

    let exe = get_exe_path()?;
    let exe_str = exe
        .to_str()
        .ok_or_else(|| "Executable path contains non-UTF-8 characters".to_string())?;

    let xml = generate_task_xml(exe_str, device_filter, delay_ms, user);

    let utf16: Vec<u16> = xml.encode_utf16().collect();
    let mut bytes = Vec::with_capacity(utf16.len() * 2 + 2);
    bytes.push(0xFF);
    bytes.push(0xFE);
    for u in utf16 {
        bytes.extend_from_slice(&u.to_le_bytes());
    }

    // Atomic exclusive temporary file creation to mitigate CWE-377 / CWE-379 symlink attacks
    let pid = std::process::id();
    let mut temp_path = None;
    for attempt in 0..20 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let candidate =
            env::temp_dir().join(format!("earplugger_task_{}_{}_{}.xml", pid, nanos, attempt));

        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                file.write_all(&bytes)
                    .map_err(|e| format!("Failed to write temporary XML file: {}", e))?;
                // Flush to OS buffers before dropping the handle so schtasks.exe reads complete data.
                file.flush()
                    .map_err(|e| format!("Failed to flush temporary XML file: {}", e))?;
                temp_path = Some(candidate);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Failed to create secure temporary file: {}", e)),
        }
    }

    let temp_xml_path =
        temp_path.ok_or_else(|| "Failed to allocate unique temporary XML file".to_string())?;
    let _guard = TempFileGuard(temp_xml_path.clone());

    // Verify the temp path is valid UTF-8 before passing to schtasks; to_string_lossy() would
    // silently corrupt non-UTF-8 paths (e.g. %TEMP% on Japanese Windows with Shift-JIS profile).
    let temp_str = temp_xml_path
        .to_str()
        .ok_or_else(|| "Temporary file path contains non-UTF-8 characters".to_string())?;

    let schtasks = get_system32_path("schtasks.exe");
    let output = Command::new(schtasks)
        .args(["/create", "/tn", TASK_NAME, "/xml", temp_str, "/f"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks.exe: {}", e))?;

    if !output.status.success() {
        let err = decode_process_output(&output.stderr);
        let out = decode_process_output(&output.stdout);
        let msg = format!("{} {}", out, err).trim().to_string();
        return Err(format!("schtasks failed: {}", msg));
    }

    Ok(())
}

pub fn uninstall_task(disable_channel: bool) -> Result<(), String> {
    let schtasks = get_system32_path("schtasks.exe");
    let output = Command::new(schtasks)
        .args(["/delete", "/tn", TASK_NAME, "/f"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks.exe: {}", e))?;

    if !output.status.success() {
        let err = decode_process_output(&output.stderr);
        let out = decode_process_output(&output.stdout);
        let msg = format!("{} {}", out, err).trim().to_string();
        return Err(format!("schtasks delete failed: {}", msg));
    }

    if disable_channel {
        disable_audio_operational_log()?;
    }

    Ok(())
}

pub struct TaskStatusDetails {
    pub is_enabled: bool,
    pub xml_raw: String,
}

pub fn parse_task_xml_status(raw: &[u8]) -> TaskStatusDetails {
    // schtasks /query /xml outputs UTF-16 LE with a BOM on Windows. Detect the BOM and
    // decode accordingly. If no BOM is present, treat as UTF-8 (future-proofs against Wine
    // or redirected output).
    let xml = if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
        let u16_data: Vec<u16> = raw[2..]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        String::from_utf16_lossy(&u16_data).trim().to_string()
    } else {
        String::from_utf8_lossy(raw).trim().to_string()
    };

    // Extract only the <Settings> block to avoid trigger-level <Enabled> tags.
    // Use relative indexing and bounds guards to prevent slicing panics on malformed XML.
    let lower_xml = xml.to_ascii_lowercase();
    let settings_block = if let Some(start) = lower_xml.find("<settings>") {
        if let Some(end_offset) = lower_xml[start..].find("</settings>") {
            &xml[start..start + end_offset]
        } else {
            &xml[start..]
        }
    } else {
        &xml[..]
    };

    // Case-insensitive check for disabled state, ignoring whitespace variations inside <Enabled>...</Enabled>
    let lower = settings_block.to_ascii_lowercase();
    let is_enabled = if let Some(e_start) = lower.find("<enabled>") {
        if let Some(e_end) = lower[e_start + 9..].find("</enabled>") {
            let val = lower[e_start + 9..e_start + 9 + e_end].trim();
            val != "false" && val != "0"
        } else {
            true
        }
    } else {
        true
    };

    TaskStatusDetails {
        is_enabled,
        xml_raw: xml,
    }
}

pub fn query_task_status() -> Result<Option<TaskStatusDetails>, String> {
    let task_file = get_system32_path(&format!("Tasks\\{}", TASK_NAME));
    let schtasks = get_system32_path("schtasks.exe");
    let output = Command::new(schtasks)
        .args(["/query", "/tn", TASK_NAME, "/xml"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks.exe: {}", e))?;

    if !output.status.success() {
        // If the task file in System32\Tasks does not exist, the task is definitely not registered.
        // This provides 100% reliable, language-independent detection without console scraping.
        if !task_file.exists() {
            return Ok(None);
        }

        // Capture both stdout and stderr since schtasks writes failure messages across both streams
        let err = decode_process_output(&output.stderr);
        let out = decode_process_output(&output.stdout);
        let combined = format!("{} {}", out, err).trim().to_string();
        if combined.is_empty() {
            return Ok(None);
        }
        return Err(format!("Query failed (schtasks: {})", combined));
    }

    Ok(Some(parse_task_xml_status(&output.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_escape_characters() {
        let input = "Audio & Video <Test> \"Quote\" 'Single'\x07\x00";
        let escaped = xml_escape(input);
        assert_eq!(
            escaped,
            "Audio &amp; Video &lt;Test&gt; &quot;Quote&quot; &apos;Single&apos;"
        );
    }

    #[test]
    fn test_format_xpath_string_literal_without_quotes() {
        let input = "RODE NT-USB";
        let formatted = format_xpath_string_literal(input);
        assert_eq!(formatted, "'RODE NT-USB'");
    }

    #[test]
    fn test_format_xpath_string_literal_with_apostrophe() {
        let input = "User's AirPods";
        let formatted = format_xpath_string_literal(input);
        assert_eq!(formatted, "\"User's AirPods\"");
    }

    #[test]
    fn test_format_xpath_string_literal_single_apostrophe() {
        let input = "'";
        let formatted = format_xpath_string_literal(input);
        assert_eq!(formatted, "\"'\"");
    }

    #[test]
    fn test_generate_task_xml_structure() {
        let xml = generate_task_xml(
            r"C:\Audio & Tools\earplugger.exe",
            Some("RODE NT-USB"),
            150,
            None,
        );
        // Command element must NOT contain quotes (&quot;)
        assert!(xml.contains("<Command>C:\\Audio &amp; Tools\\earplugger.exe</Command>"));
        assert!(xml.contains("Data[@Name=&apos;DeviceName&apos;]=&apos;RODE NT-USB&apos;"));
        assert!(xml.contains("<Arguments>restart --silent</Arguments>"));
        assert!(xml.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
        assert!(xml.contains(
            "(Data[@Name=&apos;flow&apos;]=&apos;0&apos; or Data[@Name=&apos;flow&apos;]=&apos;1&apos;)"
        ));
        assert!(xml.contains("<ExecutionTimeLimit>PT30S</ExecutionTimeLimit>"));
    }

    #[test]
    fn test_generate_task_xml_custom_delay_and_user() {
        let xml = generate_task_xml(r"C:\earplugger.exe", None, 200, Some("DOMAIN\\Alice"));
        assert!(xml.contains("<Arguments>restart --delay-ms 200 --silent</Arguments>"));
        assert!(xml.contains("<UserId>DOMAIN\\Alice</UserId>"));
    }

    #[test]
    fn test_query_task_status_utf16_decoding() {
        // Simulate UTF-16 LE BOM-prefixed output from schtasks with a disabled task.
        // The <Enabled>false</Enabled> is inside <Settings>.
        let xml_str = "<Task><Settings><Enabled>false</Enabled></Settings></Task>";
        let utf16: Vec<u16> = xml_str.encode_utf16().collect();
        let mut raw: Vec<u8> = vec![0xFF, 0xFE];
        for u in &utf16 {
            raw.extend_from_slice(&u.to_le_bytes());
        }

        let status = parse_task_xml_status(&raw);
        assert!(
            !status.is_enabled,
            "UTF-16 decoded disabled task should report is_enabled=false"
        );

        // Also test enabled task
        let enabled_xml = "<Task><Settings><Enabled>true</Enabled></Settings></Task>";
        let utf16_en: Vec<u16> = enabled_xml.encode_utf16().collect();
        let mut raw_en: Vec<u8> = vec![0xFF, 0xFE];
        for u in &utf16_en {
            raw_en.extend_from_slice(&u.to_le_bytes());
        }
        let status_en = parse_task_xml_status(&raw_en);
        assert!(
            status_en.is_enabled,
            "UTF-16 decoded enabled task should report is_enabled=true"
        );

        // Test uppercase FALSE
        let upper_xml = "<Task><Settings><Enabled>FALSE</Enabled></Settings></Task>";
        let status_upper = parse_task_xml_status(upper_xml.as_bytes());
        assert!(!status_upper.is_enabled);

        // Test whitespace inside <Enabled> tag (e.g. " false ")
        let ws_xml = "<Task><Settings><Enabled>  false  </Enabled></Settings></Task>";
        let status_ws = parse_task_xml_status(ws_xml.as_bytes());
        assert!(!status_ws.is_enabled);

        // Test newline inside <Enabled> tag
        let nl_xml = "<Task><Settings><Enabled>\r\n0\r\n</Enabled></Settings></Task>";
        let status_nl = parse_task_xml_status(nl_xml.as_bytes());
        assert!(!status_nl.is_enabled);
    }

    #[test]
    fn test_generate_task_xml_scaled_execution_time_limit() {
        let xml = generate_task_xml(r"C:\earplugger.exe", None, 30_000, None);
        assert!(xml.contains("<ExecutionTimeLimit>PT60S</ExecutionTimeLimit>"));
    }

    #[test]
    fn test_decode_process_output_utf8_and_empty() {
        assert_eq!(decode_process_output(b""), "");
        assert_eq!(decode_process_output(b"Hello World"), "Hello World");
        assert_eq!(decode_process_output(b"  trimmed  \r\n"), "trimmed");
    }

    #[test]
    fn test_decode_process_output_utf16_bom() {
        let utf16_str = "Error: File not found\r\n";
        let mut raw = vec![0xFF, 0xFE];
        for u in utf16_str.encode_utf16() {
            raw.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decode_process_output(&raw), "Error: File not found");
    }
}
