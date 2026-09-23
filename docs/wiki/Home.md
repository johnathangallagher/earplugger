# earplugger Wiki

Welcome to the **earplugger** documentation wiki.

`earplugger` is a lightweight, zero-overhead Windows utility designed to solve audio buffer desynchronization and clock drift in VB-Audio Voicemeeter when switching hardware KVM switches, hotplugging USB devices, resuming from sleep, or recovering from audio driver faults.

## Wiki Navigation
- [Architecture](Architecture.md): Deep dive into how `earplugger` intercepts Windows MMDevAPI events, manages IPC with Voicemeeter, and enforces RAII FFI safety.
- [Task Scheduler Internals](Task-Scheduler-Internals.md): Technical specification of the Windows Task Scheduler XML event subscription and trigger mechanisms.
- [Troubleshooting](Troubleshooting.md): Common diagnostic steps, event log verification, and resolution procedures for complex hardware configurations.

## Quick Links
- [Main Repository](https://github.com/johnathangallagher/earplugger)
- [Security Policy](https://github.com/johnathangallagher/earplugger/blob/main/SECURITY.md)
- [Releases](https://github.com/johnathangallagher/earplugger/releases)
