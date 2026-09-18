# Architecture & Technical Design

## Core Problem
VB-Audio Voicemeeter ties its internal processing bus and mixing loop directly to the primary hardware output endpoint assigned to Hardware Output A1. When a hardware KVM switch disconnects a USB DAC, headset, or interface, Windows reports device detachment (`CM_PROB_PHANTOM`). Upon reconnection, Windows re-enumerates the device and allocates an active WASAPI/WDM endpoint, but Voicemeeter does not automatically rebind the audio stream, leaving buffers misaligned and causing crackling, robotic distortion, or complete silence.

## Execution Flow

1. **Kernel / Audio Subsystem State Transition:**
   Windows MMDevAPI records Event ID 65 under the `Microsoft-Windows-Audio/Operational` channel. The payload contains `DeviceName`, `flow` (`0` for Render, `1` for Capture), and `NewState` (`1` for `DEVICE_STATE_ACTIVE`).

2. **Event Trigger Activation:**
   Windows Task Scheduler evaluates the XPath query against the event data. When the query matches, Task Scheduler launches `earplugger.exe restart` with `RunLevel=LeastPrivilege` under an interactive logon token.

3. **Zero-Allocation Process Detection:**
   `earplugger` takes a lightweight process snapshot using native Win32 `CreateToolhelp32Snapshot`. It traverses the process list in `<0.3ms` comparing wide character string slices without heap allocations. If Voicemeeter is not running, it exits immediately without waiting.

4. **Handshake Debounce / Settle Window:**
   USB audio class drivers, interface controllers, and firmware negotiate sample rates, buffer sizes, and clock timing over a brief transient interval. `earplugger` pauses for a configurable settle duration (default: 150ms).

5. **Voicemeeter Remote C API Dynamic Invocation:**
   `earplugger` dynamically resolves the bitness-matched DLL (`VoicemeeterRemote64.dll` on 64-bit architectures, `VoicemeeterRemote.dll` on 32-bit) using `LoadLibraryExW` with `LOAD_WITH_ALTERED_SEARCH_PATH`.
   It binds `VBVMR_Login`, `VBVMR_SetParameterFloat`, `VBVMR_GetParameterStringW`, `VBVMR_IsParametersDirty`, and `VBVMR_Logout`. It writes `Command.Restart = 1.0f` to shared memory, polls for parameter consumption, and unloads the library cleanly via RAII Drop handlers.
