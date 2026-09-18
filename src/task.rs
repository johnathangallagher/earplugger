use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const TASK_NAME: &str = "Earplugger_AutoRestart";

pub fn get_exe_path() -> Result<PathBuf, String> {
    env::current_exe().map_err(|e| format!("Failed to resolve current exe path: {}", e))
}

pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
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

pub fn sanitize_xpath_literal(s: &str) -> String {
    // Prevent XPath injection and handle quotes safely
    // Replace single quotes and control characters
    s.replace('\'', "")
}

pub fn generate_task_xml(exe_path: &str, device_filter: Option<&str>, delay_ms: u64) -> String {
    let filter_clause = match device_filter {
        Some(dev) => {
            let sanitized = sanitize_xpath_literal(dev);
            format!(
                " and *[EventData[Data[@Name='DeviceName']='{}' and Data[@Name='flow']='0' and Data[@Name='NewState']='1']]",
                sanitized
            )
        }
        None => {
            " and *[EventData[Data[@Name='flow']='0' and Data[@Name='NewState']='1']]".to_string()
        }
    };

    let subscription = format!(
        "<QueryList><Query Id=\"0\" Path=\"Microsoft-Windows-Audio/Operational\"><Select Path=\"Microsoft-Windows-Audio/Operational\">*[System[Provider[@Name='Microsoft-Windows-Audio'] and (EventID=65)]]{}</Select></Query></QueryList>",
        filter_clause
    );

    let escaped_subscription = xml_escape(&subscription);
    let escaped_exe = xml_escape(exe_path);

    let args_val = if delay_ms != 150 {
        format!("restart --delay-ms {}", delay_ms)
    } else {
        "restart".to_string()
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

pub fn install_task(device_filter: Option<&str>, delay_ms: u64) -> Result<(), String> {
    let exe = get_exe_path()?;
    let exe_str = exe.to_string_lossy();

    let xml = generate_task_xml(&exe_str, device_filter, delay_ms);
    let temp_xml_path = env::temp_dir().join("earplugger_task.xml");

    let utf16: Vec<u16> = xml.encode_utf16().collect();
    let mut bytes = Vec::with_capacity(utf16.len() * 2 + 2);
    bytes.push(0xFF);
    bytes.push(0xFE);
    for u in utf16 {
        bytes.extend_from_slice(&u.to_le_bytes());
    }

    fs::write(&temp_xml_path, &bytes)
        .map_err(|e| format!("Failed to write temporary XML file: {}", e))?;

    let output = Command::new("schtasks")
        .args([
            "/create",
            "/tn",
            TASK_NAME,
            "/xml",
            &temp_xml_path.to_string_lossy(),
            "/f",
        ])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks: {}", e))?;

    let _ = fs::remove_file(&temp_xml_path);

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        return Err(format!("schtasks failed: {} {}", out.trim(), err.trim()));
    }

    Ok(())
}

pub fn uninstall_task() -> Result<(), String> {
    let output = Command::new("schtasks")
        .args(["/delete", "/tn", TASK_NAME, "/f"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("schtasks delete failed: {}", err.trim()));
    }

    Ok(())
}

pub fn query_task_status() -> Result<String, String> {
    let output = Command::new("schtasks")
        .args(["/query", "/tn", TASK_NAME, "/fo", "LIST"])
        .output()
        .map_err(|e| format!("Failed to invoke schtasks: {}", e))?;

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
        let input = "Audio & Video <Test> \"Quote\" 'Single'";
        let escaped = xml_escape(input);
        assert_eq!(
            escaped,
            "Audio &amp; Video &lt;Test&gt; &quot;Quote&quot; &apos;Single&apos;"
        );
    }

    #[test]
    fn test_sanitize_xpath_literal() {
        let input = "User's RODE 'Special' Mic";
        let sanitized = sanitize_xpath_literal(input);
        assert_eq!(sanitized, "Users RODE Special Mic");
    }

    #[test]
    fn test_generate_task_xml_structure() {
        let xml = generate_task_xml(r"C:\Audio & Tools\earplugger.exe", Some("RODE NT-USB"), 150);
        assert!(xml.contains("<Command>C:\\Audio &amp; Tools\\earplugger.exe</Command>"));
        assert!(xml.contains("Data[@Name=&apos;DeviceName&apos;]=&apos;RODE NT-USB&apos;"));
        assert!(xml.contains("<Arguments>restart</Arguments>"));
    }

    #[test]
    fn test_generate_task_xml_custom_delay() {
        let xml = generate_task_xml(r"C:\earplugger.exe", None, 200);
        assert!(xml.contains("<Arguments>restart --delay-ms 200</Arguments>"));
    }
}
