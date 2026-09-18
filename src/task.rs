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

fn get_system32_path(binary: &str) -> PathBuf {
    let sys_root = env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(sys_root).join("System32").join(binary)
}

pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        // Filter out illegal XML 1.0 control characters
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
    } else {
        // XPath 1.0 concat() for literals containing single quotes
        let tokens: Vec<&str> = clean.split('\'').collect();
        let mut parts = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if !token.is_empty() {
                parts.push(format!("'{}'", token));
            }
            if i + 1 < tokens.len() {
                parts.push("\"'\"".to_string());
            }
        }
        // W3C XPath 1.0 section 4.2 requires concat() to take >= 2 arguments
        if parts.is_empty() {
            "''".to_string()
        } else if parts.len() == 1 {
            format!("concat({}, '')", parts[0])
        } else {
            format!("concat({})", parts.join(", "))
        }
    }
}

pub fn generate_task_xml(exe_path: &str, device_filter: Option<&str>, delay_ms: u64) -> String {
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
    <Principal id="Author">
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
    <ExecutionTimeLimit>PT1M</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
      {}
    </Exec>
  </Actions>
</Task>"#,
        escaped_subscription, escaped_exe, args_elem
    )
}

pub fn enable_audio_operational_log() -> Result<(), String> {
    let wevtutil = get_system32_path("wevtutil.exe");
    let output = Command::new(wevtutil)
        .args(["sl", "Microsoft-Windows-Audio/Operational", "/e:true"])
        .output()
        .map_err(|e| format!("Failed to invoke wevtutil.exe: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "wevtutil failed to enable Microsoft-Windows-Audio/Operational log: {} {}",
            out.trim(),
            err.trim()
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

pub fn install_task(device_filter: Option<&str>, delay_ms: u64) -> Result<(), String> {
    // Ensure the Windows Audio Operational event channel is active
    enable_audio_operational_log()?;

    let exe = get_exe_path()?;
    let exe_str = exe.to_string_lossy();

    let xml = generate_task_xml(&exe_str, device_filter, delay_ms);

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

    let schtasks = get_system32_path("schtasks.exe");
    let output = Command::new(schtasks)
        .args([
            "/create",
            "/tn",
            TASK_NAME,
            "/xml",
            &temp_xml_path.to_string_lossy(),
            "/f",
        ])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks.exe: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        return Err(format!("schtasks failed: {} {}", out.trim(), err.trim()));
    }

    Ok(())
}

pub fn uninstall_task() -> Result<(), String> {
    let schtasks = get_system32_path("schtasks.exe");
    let output = Command::new(schtasks)
        .args(["/delete", "/tn", TASK_NAME, "/f"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks.exe: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "schtasks delete failed: {} {}",
            out.trim(),
            err.trim()
        ));
    }

    Ok(())
}

pub fn query_task_status() -> Result<String, String> {
    let schtasks = get_system32_path("schtasks.exe");
    let output = Command::new(schtasks)
        .args(["/query", "/tn", TASK_NAME, "/fo", "LIST"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks.exe: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Task is not registered".to_string())
    }
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
        assert_eq!(formatted, "concat('User', \"'\", 's AirPods')");
    }

    #[test]
    fn test_format_xpath_string_literal_single_apostrophe_arity() {
        let input = "'";
        let formatted = format_xpath_string_literal(input);
        // Must satisfy XPath 1.0 concat() arity of >= 2
        assert_eq!(formatted, "concat(\"'\", '')");
    }

    #[test]
    fn test_generate_task_xml_structure() {
        let xml = generate_task_xml(r"C:\Audio & Tools\earplugger.exe", Some("RODE NT-USB"), 150);
        // Command element must NOT contain quotes (&quot;)
        assert!(xml.contains("<Command>C:\\Audio &amp; Tools\\earplugger.exe</Command>"));
        assert!(xml.contains("Data[@Name=&apos;DeviceName&apos;]=&apos;RODE NT-USB&apos;"));
        assert!(xml.contains("<Arguments>restart --silent</Arguments>"));
        assert!(xml.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
        assert!(xml.contains("(Data[@Name=&apos;flow&apos;]=&apos;0&apos; or Data[@Name=&apos;flow&apos;]=&apos;1&apos;)"));
    }

    #[test]
    fn test_generate_task_xml_custom_delay() {
        let xml = generate_task_xml(r"C:\earplugger.exe", None, 200);
        assert!(xml.contains("<Arguments>restart --delay-ms 200 --silent</Arguments>"));
    }
}
