use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const TASK_NAME: &str = "Earplugger_AutoRestart";

pub fn get_exe_path() -> Result<PathBuf, String> {
    env::current_exe().map_err(|e| format!("Failed to resolve current exe path: {}", e))
}

pub fn generate_task_xml(exe_path: &str, device_filter: Option<&str>, delay_ms: u64) -> String {
    let filter_clause = match device_filter {
        Some(dev) => format!(
            " and *[EventData[Data[@Name='DeviceName']='{}' and Data[@Name='flow']='0' and Data[@Name='NewState']='1']]",
            dev
        ),
        None => {
            " and *[EventData[Data[@Name='flow']='0' and Data[@Name='NewState']='1']]".to_string()
        }
    };

    let subscription = format!(
        "<QueryList><Query Id=\"0\" Path=\"Microsoft-Windows-Audio/Operational\"><Select Path=\"Microsoft-Windows-Audio/Operational\">*[System[Provider[@Name='Microsoft-Windows-Audio'] and (EventID=65)]]{}</Select></Query></QueryList>",
        filter_clause
    );

    let escaped_subscription = subscription
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");

    let args = if delay_ms != 150 {
        format!("<Arguments>restart --delay-ms {}</Arguments>", delay_ms)
    } else {
        "<Arguments>restart</Arguments>".to_string()
    };

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
        escaped_subscription, exe_path, args
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
