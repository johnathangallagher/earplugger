# earplugger Wiki

Welcome to the **earplugger** documentation wiki.

`earplugger` is a lightweight, zero-overhead Windows utility designed to solve audio buffer desynchronization and clock drift in VB-Audio Voicemeeter when switching hardware KVM switches, USB hubs, or audio endpoints.

## Wiki Navigation
- [[Architecture]]: Deep dive into how `earplugger` intercepts Windows MMDevAPI events, manages IPC with Voicemeeter, and enforces RAII FFI safety.
- [[Task-Scheduler-Internals]]: Technical specification of the Windows Task Scheduler XML event subscription and trigger mechanisms.
- [[Troubleshooting]]: Common diagnostic steps, event log verification, and resolution procedures for complex hardware configurations.

## Quick Links
- [Main Repository](https://github.com/johnathangallagher/earplugger)
- [Security Policy](https://github.com/johnathangallagher/earplugger/blob/main/SECURITY.md)
- [Releases](https://github.com/johnathangallagher/earplugger/releases)
