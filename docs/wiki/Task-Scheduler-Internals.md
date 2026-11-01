# Task Scheduler Internals & Event Subscriptions

## Windows Event Log Channels
`earplugger` subscribes to:
1. **Audio Reconnect & Driver Crash Trigger**:
   - **Channel**: `Microsoft-Windows-Audio/Operational`
   - **Provider**: `Microsoft-Windows-Audio`
   - **Event IDs**:
     - `65` (Audio device state transition to ACTIVE upon USB/KVM reconnect)
     - `4` (Windows Audio Device Graph Isolation / driver error recovery)
2. **Sleep / Resume Trigger** (enabled by default; toggleable via `--no-wake`):
   - **Channel**: `System`
   - **Providers & Event IDs**:
     - `Microsoft-Windows-Power-Troubleshooter` (Event ID `1` — system returned from low power state)
     - `Microsoft-Windows-Kernel-Power` (Event IDs `107`, `507`, and `566` — resume from sleep, modern standby, or display signal return)
3. **Session Unlock & KVM Console Connect Trigger** (enabled by default; toggleable via `--no-wake`):
   - **Trigger Type**: `<SessionStateChangeTrigger>`
   - **Events**:
     - `SessionUnlock` — user unlocks workstation upon returning to desk
     - `ConsoleConnect` — user flips KVM switch console back to this PC

> [!NOTE]
> On default installations of Windows 10 and Windows 11, the `Microsoft-Windows-Audio/Operational` event channel is disabled. The `earplugger install` command automatically activates it using `%SystemRoot%\System32\wevtutil.exe sl Microsoft-Windows-Audio/Operational /e:true`. The `System` channel is part of core Windows NT logging and is always active.

## Event Subscription XPath Structure

### Audio Device Reconnect & Driver Crash Subscription
```xml
<QueryList>
  <Query Id="0" Path="Microsoft-Windows-Audio/Operational">
    <Select Path="Microsoft-Windows-Audio/Operational">
      *[System[Provider[@Name='Microsoft-Windows-Audio'] and (EventID=65)]]
      and *[EventData[(Data[@Name='DeviceName']='Sennheiser 560S (2- RODE NT-USB)' or Data[@Name='DeviceName']='RODE NT-USB' or Data[@Name='DeviceName']='2- RODE NT-USB' or Data[@Name='DeviceName']='Sennheiser 560S') and (Data[@Name='flow']='0' or Data[@Name='flow']='1')]]
    </Select>
    <Select Path="Microsoft-Windows-Audio/Operational">
      *[System[Provider[@Name='Microsoft-Windows-Audio'] and (EventID=4)]]
    </Select>
  </Query>
</QueryList>
```
The query subscribes to both device reconnects (`EventID=65`) and Windows Audio Device Graph Isolation / driver crashes (`EventID=4`), automatically restarting the Voicemeeter audio engine if `audiodg.exe` crashes or is restarted by Windows.

### Sleep / Wake Resume Subscription
```xml
<QueryList>
  <Query Id="0" Path="System">
    <Select Path="System">*[System[Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and (EventID=1)]]</Select>
    <Select Path="System">*[System[Provider[@Name='Microsoft-Windows-Kernel-Power'] and (EventID=107 or EventID=507 or EventID=566)]]</Select>
  </Query>
</QueryList>
```

### Workstation Unlock & KVM Console Connect Triggers
```xml
<SessionStateChangeTrigger>
  <Enabled>true</Enabled>
  <StateChange>SessionUnlock</StateChange>
</SessionStateChangeTrigger>
<SessionStateChangeTrigger>
  <Enabled>true</Enabled>
  <StateChange>ConsoleConnect</StateChange>
</SessionStateChangeTrigger>
```

### XPath Literal Sanitization & Multi-Candidate Matching
Device names can contain single quotes or apostrophes (for example: `User's AirPods`). Because the Windows Event Log query engine (`wevtapi.dll`) implements a restricted subset of XPath 1.0 where functions like `concat()` are unsupported (producing error `15008`), `earplugger` formats strings containing apostrophes using double-quoted string literals:
`Data[@Name='DeviceName']="User's AirPods"` (which is XML-escaped to `Data[@Name=&apos;DeviceName&apos;]=&quot;User&apos;s AirPods&quot;`).

Strings containing both single and double quotes are rejected during CLI validation because `wevtapi.dll` does not support escaping quotes or `concat()` within event subscription queries.

When an endpoint friendly name contains parenthetical roles or custom user renames (for example: `Speakers (RODE NT-USB)` or `Sennheiser 560S (2- RODE NT-USB)`), `earplugger` extracts all candidate permutations into an `or` chain:
1. Full composite friendly name (`Sennheiser 560S (2- RODE NT-USB)`)
2. Bare hardware adapter name without instance prefix (`RODE NT-USB`)
3. Hardware adapter name with Windows endpoint instance index (`2- RODE NT-USB`)
4. Custom user friendly name (`Sennheiser 560S`)

This guarantees trigger activation regardless of whether Windows MMDevAPI logs the custom friendly name, the bare hardware name, or the composite endpoint string in Event 65.

Control characters (`< 0x20`) are stripped to preserve XML parser validity.

## Task Settings & Queuing
- **MultipleInstancesPolicy**: `StopExisting`
  Acts as an automatic hardware debouncer. If rapid bursts of endpoint re-enumeration events fire (e.g. monitor sleep/wake, KVM toggle, or multiple render/capture endpoints registering in quick succession), each subsequent event terminates the pending instance and resets the settle delay timer. This guarantees Voicemeeter restarts only once, after the final audio device has completely settled.
- **ExecutionTimeLimit**: Dynamically scaled based on `--delay-ms` and retry cycles (`PT32S` minimum for default 150ms delay with 3 cycles @ 1000ms, scaling up to accommodate configured delays and cycles to allow the full settle delay and multi-cycle Voicemeeter restart sequence to complete before termination).
- **DisallowStartIfOnBatteries**: `false` (operates normally on laptops).
- **RunLevel**: `LeastPrivilege` (runs under user logon context without elevated tokens).
