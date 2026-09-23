# earplugger

> [!NOTE]
> **Disclaimer:** This project was created and written by an AI / Large Language Model (LLM). While built and tested for reliability, please review the code and configuration before deploying in your environment.

Automatically restarts the Voicemeeter audio engine when a USB audio device or KVM switch reconnects on Windows, when resuming from sleep, or after an audio driver / engine crash.

[![License: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/License-PolyForm%20Noncommercial-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-0078D6.svg)](#)
[![Rust](https://img.shields.io/badge/Language-Rust%202024-DEA584.svg)](https://www.rust-lang.org/)
[![Latest Release](https://img.shields.io/github/v/release/johnathangallagher/earplugger?include_prereleases&color=brightgreen)](https://github.com/johnathangallagher/earplugger/releases/latest)

---

## Why this exists

When using **Voicemeeter** (Standard, Banana, or Potato) with a **KVM switch**, USB audio switch, or hotpluggable DAC/headset:

1. Voicemeeter locks its master clock and mixing bus to the device configured under **Hardware Output A1**.
2. Switching the KVM disconnects the USB device, invalidating the active audio stream.
3. Switching back reconnects the device, but Voicemeeter often fails to re-acquire the stream cleanly—leaving the audio distorted, crackling, or completely silent.
4. Similarly, when resuming the PC from sleep or when the Windows Audio service (`audiodg.exe`) / driver recovers from a fault, stream handles break and require a restart.
5. The usual fix is manually opening Voicemeeter and hitting `Menu -> Restart Audio Engine` (`Ctrl + R`) every time.

`earplugger` automates this. It listens for the Windows reconnection, wake, or driver crash event, waits a moment for the USB handshake to settle, and sends a restart signal directly to Voicemeeter via its native API.

No background processes. No system tray clutter. 0 MB RAM and 0% CPU when idle.

---

## How it works

```mermaid
sequenceDiagram
    participant Source as KVM Switch / Wake / Driver Crash
    participant Win as Windows Subsystems (Audio & Power)
    participant Task as Windows Task Scheduler
    participant EP as earplugger (Rust)
    participant VM as Voicemeeter

    Source->>Win: USB reconnect / wake / audiodg.exe fault
    Win->>Win: Logs Event (ID 65, 4, 1, 107, 507)
    Win->>Task: Event trigger fires
    Task->>EP: Runs earplugger restart (windowless)
    EP->>EP: Waits 150ms for USB handshake to settle
    EP->>VM: Native IPC restart via VoicemeeterRemote64.dll
    VM->>VM: Audio engine restarts & resyncs A1 clock
    EP-->>Task: Exits cleanly
```

1. **Event triggers:** Subscribes to device reconnects (`Event ID 65`) and audio engine / driver crashes (`Event ID 4`) in `Microsoft-Windows-Audio/Operational`, plus system sleep/wake resume events in `System` (`Power-Troubleshooter Event ID 1`, `Kernel-Power Event IDs 107 & 507`).
2. **Multi-candidate device filter:** Automatically generates an XPath `OR` filter matching full friendly names (`"Sennheiser 560S (2- RODE NT-USB)"`), bare hardware names (`"RODE NT-USB"`), custom user renames (`"Sennheiser 560S"`), and endpoint instance prefixes (`"2- RODE NT-USB"`), guaranteeing trigger activation regardless of how Windows formats Event 65.
3. **Handshake debounce:** Pauses for a configurable 150ms so Windows audio drivers finish enumerating before restarting the engine.
4. **Native IPC:** Connects directly to Voicemeeter's shared memory API (`VoicemeeterRemote64.dll` or `VoicemeeterRemote.dll`), sets `Command.Restart = 1.0`, confirms the command was received, and unloads.

---

## Installation

### 1. Download or Build

- **Prebuilt binary:** Download `earplugger.exe` from [Releases](https://github.com/johnathangallagher/earplugger/releases/latest).
- **From source:**
  ```bash
  git clone https://github.com/johnathangallagher/earplugger.git
  cd earplugger
  cargo build --release
  ```

### 2. Register the Task

Open an **Administrator** terminal and run:

```powershell
earplugger.exe install
```

This will:
- Auto-detect your currently running Voicemeeter instance and active **Hardware A1 device**.
- Enable the Windows Audio operational event log channel (`wevtutil sl Microsoft-Windows-Audio/Operational /e:true`).
- Register the `Earplugger_AutoRestart` event trigger in Windows Task Scheduler.

When working from a source clone, you can also right-click `src\install.bat` and select **Run as administrator**.

#### Custom Options

```powershell
# Specify a device name manually (if Voicemeeter isn't running during install):
earplugger.exe install --device "RODE NT-USB"

# Change the post-reconnect settling delay (default 150ms, max 30000ms):
earplugger.exe install --delay-ms 250

# Bind the task to a specific user account:
earplugger.exe install --user "DOMAIN\User"

# Disable sleep/wake resume triggers:
earplugger.exe install --no-wake
```

### 3. Verify

Check that the scheduled task and Voicemeeter connection are ready:

```powershell
earplugger.exe status
```

---

## CLI Reference

```text
Usage: earplugger <COMMAND> [OPTIONS]

Commands:
  restart              Wait for device settle and restart Voicemeeter engine (default)
  install              Register the Windows Task Scheduler event trigger
  uninstall            Remove the Windows Task Scheduler event trigger
  status               Check status of task trigger and Voicemeeter engine
  version              Print version information (--version, -v, -V)
  help                 Print this message

Options for 'restart':
  --delay-ms <MS>      Settling delay in milliseconds before restart (default: 150, max: 30000)
  --silent             Suppress interactive output (used by Task Scheduler)

Options for 'install':
  --device <NAME>      Audio device name filter. If omitted, auto-detects A1 from Voicemeeter.
  --delay-ms <MS>      Settling delay in milliseconds to configure in the trigger (default: 150, max: 30000)
  --user <USERNAME>    Target user for scheduled task (e.g. DOMAIN\User)
  --no-wake            Disable triggers on system wake/resume from sleep (enabled by default)
  --wake               Enable triggers on system wake/resume from sleep

Options for 'uninstall':
  --disable-channel    Also disable the Microsoft-Windows-Audio/Operational event channel
```

---

## Uninstallation

To remove the scheduled task:

```powershell
earplugger.exe uninstall
```

Or if using the source checkout, right-click `src\uninstall.bat` and select **Run as administrator**.

To also turn off the Windows Audio operational event channel:

```powershell
earplugger.exe uninstall --disable-channel
```

---

## Building & Testing

Requires Rust 1.85.0+ (Edition 2024).

```bash
cargo test
cargo build --release
```

---

## License

Licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE).
