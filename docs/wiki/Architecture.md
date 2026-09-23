# Architecture & Technical Design

## Core Problem
VB-Audio Voicemeeter ties its internal processing bus and mixing loop directly to the primary hardware output endpoint assigned to Hardware Output A1. Stream handles become invalid or desynchronized across three primary system scenarios:
1. **KVM Switch / USB Hotplug**: When a hardware switch disconnects a USB DAC, headset, or interface, Windows reports device detachment (`CM_PROB_PHANTOM`). Upon reconnection, Windows re-enumerates the device and allocates an active WASAPI/WDM endpoint, but Voicemeeter does not automatically rebind the audio stream, leaving buffers misaligned and causing crackling, robotic distortion, or complete silence.
2. **Audio Driver / Engine Crashes**: When an audio driver faults or the Windows Audio Device Graph Isolation service (`audiodg.exe`) terminates and restarts, Voicemeeter's active stream binding is severed.
3. **System Sleep / Resume**: Resuming from modern standby (S0ix) or traditional S3 sleep often re-initializes USB controllers and audio interfaces out-of-order, corrupting Voicemeeter's master clock reference.

## Execution Flow

1. **System & Audio Subsystem State Transitions:**
   - **Audio Reconnect**: Windows MMDevAPI records Event ID 65 under `Microsoft-Windows-Audio/Operational` (`flow`: `0` for Render, `1` for Capture; `NewState`: `1` for `DEVICE_STATE_ACTIVE`).
   - **Driver / Engine Crash**: Windows records Event ID 4 under `Microsoft-Windows-Audio/Operational` when `audiodg.exe` or an audio driver encounters an error condition.
   - **Sleep / Resume**: Windows records Event ID 1 under `System` (`Microsoft-Windows-Power-Troubleshooter`) or Event IDs 107/507 (`Microsoft-Windows-Kernel-Power`).

2. **Event Trigger Activation & Multi-Candidate Matching:**
   Windows Task Scheduler evaluates the configured XPath subscription query against the incoming event. For device reconnects, multi-candidate permutation matching ensures triggers fire whether Windows logs the composite friendly name, custom user alias, or bare hardware descriptor. When matched, Task Scheduler launches `earplugger.exe restart --silent` with `RunLevel=LeastPrivilege` under the interactive user logon token.

3. **Zero-Allocation Process Detection:**
   `earplugger` takes a lightweight process snapshot using native Win32 `CreateToolhelp32Snapshot`. It traverses the process list in `<0.3ms` comparing wide character string slices without heap allocations. If Voicemeeter is not running, it exits immediately without waiting.

4. **Handshake Debounce / Settle Window:**
   USB audio class drivers, interface controllers, and firmware negotiate sample rates, buffer sizes, and clock timing over a brief transient interval. `earplugger` pauses for a configurable settle duration (default: 150ms).

5. **Voicemeeter Remote C API Dynamic Invocation:**
   `earplugger` dynamically resolves the bitness-matched DLL (`VoicemeeterRemote64.dll` on 64-bit architectures, `VoicemeeterRemote.dll` on 32-bit) using `LoadLibraryExW` with `LOAD_WITH_ALTERED_SEARCH_PATH`.
   It binds `VBVMR_Login`, `VBVMR_SetParameterFloat`, `VBVMR_GetParameterStringW`, `VBVMR_IsParametersDirty`, and `VBVMR_Logout`. It writes `Command.Restart = 1.0f` to shared memory, polls for parameter consumption, and unloads the library cleanly via RAII Drop handlers.
