# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-09-18

### Security
- Implemented atomic exclusive temporary file creation using `create_new(true)` with high-resolution timestamp and PID isolation in `%TEMP%`, mitigating symlink and junction hijacking attacks (CWE-377 / CWE-379).
- Fully qualified all administrative utility executions (`schtasks.exe`, `wevtutil.exe`) using `%SystemRoot%\System32` to prevent untrusted search path binary execution (CWE-426).
- Standardized Task Scheduler `<Command>` executable path definition to conform to Windows Task Scheduler XML schema specifications.
- Switched dynamic library loading to `LoadLibraryExW` with `LOAD_WITH_ALTERED_SEARCH_PATH` to prevent directory-relative DLL preloading vulnerabilities.
- Replaced deprecated `IsUserAnAdmin` with modern Win32 `OpenProcessToken` and `GetTokenInformation` `TokenElevation` checks.
- Switched project license to PolyForm Noncommercial License 1.0.0.

### Fixed
- Fixed FFI ABI signature mismatch for `VBVMR_GetParameterStringW` by adding the required 3rd buffer length argument (`size: i32`), preventing stack frame corruption and undefined behavior.
- Fixed sign-extension for `HKEY_LOCAL_MACHINE` constant (`(-2147483646isize) as *mut c_void`), resolving `ERROR_INVALID_HANDLE` failures on 64-bit Windows registry queries.
- Fixed undefined behavior in registry deserialization by allocating directly into an aligned `Vec<u16>` buffer and validating `REG_SZ` / `REG_EXPAND_SZ` types.
- Fixed typo in Voicemeeter Banana 64-bit process matching name (`voicemeeterpro_x64.exe`), restoring auto-restart detection.
- Fixed `VBVMR_Login()` error handling to distinguish code `0` (success) from code `1` (Voicemeeter not launched) and negative error returns.
- Fixed XPath generation to preserve single quotes and apostrophes in device names via XPath 1.0 `concat()`, enforcing the minimum 2-argument arity required by W3C specifications.
- Fixed device name parsing to extract hardware adapters while preserving embedded trademarks (e.g. `Realtek(R) Audio`, `Intel(R) Display Audio`).
- Automatically enable the `Microsoft-Windows-Audio/Operational` event log channel during installation via `wevtutil.exe` to guarantee Event 65 records on clean Windows installations.
- Fixed exit code in `uninstall` command to return status code `1` on failure and combined `stdout`/`stderr` reporting so errors are never blank.
- Added check for Voicemeeter presence prior to sleeping in `restart` command to prevent idle blocking when Voicemeeter is offline.
- Added `--help` / `-h` handling across subcommands and prevented accidental task deletion when running `uninstall --help`.

### Changed
- Converted process snapshot matching in `is_voicemeeter_running()` to zero-allocation UTF-16 slice comparison against string literals.
- Added bounds checking for `--delay-ms` (`0 <= delay <= 30000`) and support for `--flag=value` syntax.
- Updated event filter to monitor both playback (`flow='0'`) and capture (`flow='1'`) endpoints, with `IgnoreNew` policy to prevent double-restart storms.
- Added `--silent` flag and `FreeConsole()` detachment for background Task Scheduler runs.
- Synchronized Voicemeeter client cache with dirty polls before reading device parameters.
- Harmonized author email to `johnathangallagherusa@gmail.com` across all project files.

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
