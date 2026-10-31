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

### 2. Device Name Matching & Custom Renames
If your device has complex, driver-aliased, or user-customized friendly names (e.g. `"Sennheiser 560S (2- RODE NT-USB)"`):
1. `earplugger` automatically generates multi-candidate XPath filters covering:
   - Full composite friendly name (`"Sennheiser 560S (2- RODE NT-USB)"`)
   - Bare hardware name (`"RODE NT-USB"`)
   - Endpoint instance index (`"2- RODE NT-USB"`)
   - Custom friendly name (`"Sennheiser 560S"`)
2. To verify matched filters, run:
   ```powershell
   earplugger status
   ```
   Or inspect the Task Scheduler XML directly:
   ```powershell
   schtasks /query /tn "Earplugger_AutoRestart" /xml
   ```
3. If Windows logs an unexpected string in Event 65:
   - Unplug and replug your device.
   - Open Event Viewer -> Applications and Services Logs -> Microsoft -> Windows -> Audio -> Operational.
   - Locate Event ID 65 and inspect the XML tab for `<Data Name="DeviceName">`.
   - Re-register with the exact name or use a wildcard:
     ```powershell
     earplugger install --device "Your Exact Device Name"
     # Or install a wildcard trigger that catches any active audio endpoint:
     earplugger install --device "*"
     ```

### 3. Insufficient Settling Time or Slow USB Enumeration
Some USB audio interfaces and DACs (such as RODE NT-USB or Sennheiser 560S) require longer handshake times or multiple initialization pulses to negotiate master sample rates and stabilize clock PLLs after sleep or KVM reconnect:
```powershell
# Increase initial settle delay:
earplugger install --delay-ms 1000

# Increase number of restart pulses and interval between pulses:
earplugger install --delay-ms 1000 --cycles 5 --cycle-delay-ms 1500
```

### 4. Audio Driver / audiodg.exe Crash Recovery
`earplugger` automatically subscribes to Event ID 4 under `Microsoft-Windows-Audio/Operational` to recover Voicemeeter when the Windows audio engine or driver crashes.
- If audio becomes distorted without a physical reconnect, inspect Event Viewer under `Microsoft-Windows-Audio/Operational` for Event ID 4 (`Windows Audio Device Graph Isolation` error).
- Ensure `earplugger status` reports `Audio reconnect : Events 65, 4 (Active)`.

### 5. Sleep, Standby, & Workstation Unlock Triggers
By default, `earplugger` registers wake and session transition triggers:
- `Microsoft-Windows-Power-Troubleshooter` (Event ID 1 — system wake)
- `Microsoft-Windows-Kernel-Power` (Event IDs 107, 507, and 566 — wake, modern standby, display signal resume)
- Session state change triggers (`SessionUnlock` upon unlocking desktop, `ConsoleConnect` upon KVM switch toggle)
- Ensure `earplugger status` reports both `Sleep/resume : Events 1, 107, 507, 566 (Active)` and `Session / KVM : SessionUnlock, ConsoleConnect (Active)`. If previously installed with `--no-wake`, re-enable with:
  ```powershell
  earplugger install --wake
  ```

### 6. Manual Testing
You can manually test whether `earplugger` can communicate with Voicemeeter at any time by running:
```powershell
earplugger restart
```
This will log in via the Voicemeeter Remote API and trigger the restart sequence interactively.
