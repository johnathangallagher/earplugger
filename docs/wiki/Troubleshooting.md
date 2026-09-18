# Troubleshooting & Diagnostics

## Common Issues & Diagnoses

### 1. Voicemeeter Does Not Restart When KVM Switches
- **Verify Operational Event Channel:**
  Open PowerShell as Administrator and run:
  ```powershell
  wevtutil gl Microsoft-Windows-Audio/Operational
  ```
  Ensure `enabled: true`. If false, run `earplugger install` or enable it manually with:
  ```powershell
  wevtutil sl Microsoft-Windows-Audio/Operational /e:true
  ```

- **Verify Task Registration:**
  Run:
  ```powershell
  earplugger status
  ```
  Ensure `Earplugger_AutoRestart` displays as `REGISTERED & READY`.

### 2. Device Name Mismatch
If your device has complex or driver-aliased names:
1. Unplug and replug your device.
2. Open Windows Event Viewer -> Applications and Services Logs -> Microsoft -> Windows -> Audio -> Operational.
3. Locate Event ID 65 and inspect the XML tab for `<Data Name="DeviceName">`.
4. Register `earplugger` with the exact device name:
   ```powershell
   earplugger install --device "Your Exact Device Name"
   ```

### 3. Insufficient Settling Time
Some USB audio interfaces (such as certain multi-channel interfaces or Bluetooth dongles) require longer handshake times to negotiate master sample rates. Increase the settling delay:
```powershell
earplugger install --delay-ms 300
```

### 4. Manual Testing
You can manually test whether `earplugger` can communicate with Voicemeeter at any time by running:
```powershell
earplugger restart
```
This will log in via the Voicemeeter Remote API and trigger the restart sequence interactively.
