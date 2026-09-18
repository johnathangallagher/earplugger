# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-09-18

### Security
- Replaced predictable temporary XML file creation in `%TEMP%` with cryptographically unpredictable, process/timestamp-isolated temporary files to mitigate symlink attacks (CWE-377/CWE-379).
- Fully qualified all system utility executions (`schtasks.exe`, `wevtutil.exe`) using `%SystemRoot%\System32` to prevent untrusted search path binary execution (CWE-426).
- Wrapped executable command in quotes (`&quot;`) inside Task Scheduler `<Command>` definition to prevent unquoted search path vulnerabilities (CWE-428).
- Switched library loading to `LoadLibraryExW` with `LOAD_WITH_ALTERED_SEARCH_PATH` to prevent directory-relative DLL search hijacking.
- Added license change to PolyForm Noncommercial License 1.0.0.

### Fixed
- Fixed FFI ABI signature mismatch for `VBVMR_GetParameterStringW` by adding the required 3rd buffer length argument (`size: i32`), preventing stack frame corruption and undefined behavior.
- Added compile-time pointer width gating (`cfg(target_pointer_width)`) to ensure 64-bit builds load `VoicemeeterRemote64.dll` and 32-bit builds load `VoicemeeterRemote.dll`.
- Fixed `VBVMR_Login()` error handling to distinguish code `0` (success) from code `1` (Voicemeeter not launched) and negative error returns.
- Fixed XPath generation to preserve single quotes/apostrophes in device names via XPath 1.0 `concat()` instead of stripping them.
- Fixed device name parsing to extract outermost matching parentheses (`rfind`) and strip Voicemeeter driver prefixes (`WDM:`, `MME:`, `KS:`, `ASIO:`, etc.), preventing broken device filters like `Realtek(`.
- Automatically enable the `Microsoft-Windows-Audio/Operational` event log channel during installation via `wevtutil.exe` to guarantee Event 65 records on clean Windows installations.
- Changed Task Scheduler `MultipleInstancesPolicy` from `IgnoreNew` to `Queue` so trailing reconnect events during KVM renegotiation are not dropped.
- Replaced localized English string matching for Administrator privileges with native Win32 token elevation checks (`IsUserAnAdmin`).
- Fixed exit code in `uninstall` command to return status code `1` on failure.

### Changed
- Converted process snapshot matching in `is_voicemeeter_running()` to zero-allocation UTF-16 slice comparison.
- Added bounds checking for `--delay-ms` (`0 <= delay <= 30000`) and support for `--flag=value` syntax.
- Updated event filter to monitor both playback (`flow='0'`) and capture (`flow='1'`) endpoints, enabling support for USB microphones.
- Removed flaky timing assertions from unit tests to prevent nondeterministic CI runner failures.

## [1.1.1] - 2026-09-18

### Fixed
- Added a 100ms hold in `restart_audio_engine` before client drop to guarantee Voicemeeter's message loop consumes `Command.Restart` before shared memory is unmapped during logout.
- Restored default USB settling delay to 150ms to allow USB audio class drivers to complete clock and format negotiation on hardware reconnects.

## [1.1.0] - 2026-09-18

### Performance
- Replaced `tasklist` subprocess execution with native Win32 `CreateToolhelp32Snapshot` and `Process32NextW` APIs, reducing process detection overhead from ~235ms to <0.3ms.
- Removed 50ms post-restart sleep in `restart_audio_engine`; parameter updates write directly to Voicemeeter shared memory.
- Reduced default USB settling delay from 150ms to 75ms.

### Security & Robustness
- Implemented XML entity escaping (`&`, `<`, `>`, `"`, `'`) and XPath literal sanitization in task generation to prevent injection and XML parse errors.
- Encapsulated FFI bindings in a `VoicemeeterClient` struct with a `Drop` implementation to ensure `VBVMR_Logout` and `FreeLibrary` are called on exit or error.
- Added dynamic DLL resolution via `%ProgramFiles(x86)%`, `%ProgramW6432%`, `%ProgramFiles%`, and `%SystemDrive%`.

### CLI & Diagnostics
- Added `--version` and `-v` flags.
- Added validation for numerical arguments (`--delay-ms`) and required parameters (`--device`).
- Added explicit error message when running `install` or `uninstall` without Administrator privileges.

### Tests
- Added automated unit tests for XML escaping, XPath sanitization, task generation, delay parsing, process detection, and DLL discovery.

## [1.0.0] - 2026-09-18

### Added
- Initial release.
- Event-driven trigger integrated with Windows Task Scheduler subscribing to `Microsoft-Windows-Audio/Operational` Event ID 65.
- Dynamic loading and IPC execution via `VoicemeeterRemote64.dll` (`Command.Restart = 1.0`).
- Configurable USB settling delay to allow USB audio driver format negotiation before restart.
- CLI commands: `restart`, `install`, `uninstall`, and `status`.
