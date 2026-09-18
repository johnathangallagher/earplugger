# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-09-18

### Fixed

- Hardened `install.bat` against command injection and double expansion (CWE-78 / CWE-88) by migrating to `EnableDelayedExpansion`, eliminating `call set`, stripping duplicate outer quotes, and disabling `eol` comment parsing in numeric validation.
- Preserved user-specified `--device` argument verbatim in `handle_install`, preventing `clean_device_name` from altering explicit user input and breaking Event 65 exact XPath matching.
- Restricted `clean_device_name` parenthetical extraction to recognized audio endpoint roles (`ENDPOINT_ROLES`), preventing hardware devices with parenthetical qualifiers (e.g. `(SST)`, `(Generic)`) from being truncated.
- Replaced fragile localized string scraping in `handle_status` with language-independent `System32\Tasks` file verification and structured `Option<TaskStatusDetails>` query results.
- Added UTF-16 LE BOM detection and checked length cast in `decode_process_output`.
- Fixed `parse_uninstall_string_dir` to handle unquoted registry paths with `.exe` in arguments and verified candidate file existence on disk.
- Enforced non-ASCII input rejection at runtime in `eq_ignore_ascii_case_wide_str` across all build profiles.
- Rejected all ASCII control characters (`is_ascii_control()`) in CLI option parsing for `--device` and `--user`.
- Added `checks: write` permissions and Rust toolchain setup to `.github/workflows/security.yml` with `rustsec/audit-check@v2.0.0`.
- Expanded MSRV CI workflow to matrix-test both `x86_64` and `i686` targets with pinned toolchain actions.
- Added multi-instance numeric suffix stripping in `clean_device_name` for duplicate Windows endpoints (e.g. `Speakers (RODE NT-USB) (1)`).
- Guarded `handle_install` against registering empty device filters (`DeviceName=''`) when prefix stripping leaves an empty string, falling back to wildcard matching with an explicit warning.
- Extracted `parse_task_xml_status` into a shared function called by both `query_task_status` and unit tests, eliminating test mock duplication.
- Hardened `parse_task_xml_status` with relative indexing and bounds guards to prevent slicing panics on malformed XML, and made `<Enabled>` tag inspection case-insensitive.
- Preserved `schtasks.exe` exit code in `uninstall.bat` when the binary is absent, preventing secondary `wevtutil` execution from masking uninstallation results.
- Added detailed error diagnostics to `earplugger status` instead of unconditionally displaying `NOT REGISTERED` on query failure.
- Fixed `query_task_status` to properly decode `schtasks /query /xml` UTF-16 LE output (BOM-detected).
- Fixed `data_len` reset before `ERROR_MORE_DATA` retry in `RegQueryValueExW`.
- Fixed `char_count` computation in registry read to clamp to `buf.len()`, preventing TOCTOU buffer panics.
- Fixed `ExpandEnvironmentStringsW` to receive an explicit null-terminated copy of the logical string and explicitly locate NUL in output.
- Fixed `VBVMR_Login` to call `VBVMR_Logout` unconditionally on any non-zero return code.
- Fixed `parse_uninstall_string_dir` to use ASCII-safe `.exe` byte search, eliminating `to_lowercase()` index panic vectors.
- Fixed `get_system32_path` to dynamically retry `GetSystemDirectoryW` on buffer truncation.
- Replaced `.to_string_lossy()` with `.to_str()` for temporary XML paths.
- Added ASCII control character validation in `parse_options` for `--device` and `--user`.
- Delegated direct flag invocations (e.g. `earplugger --silent`) to the default `restart` command in `main()`.
- Separated release packaging from release publication in `.github/workflows/release.yml` with a downstream `publish` job, eliminating concurrent release publishing race conditions.
- Updated `docs/wiki/Task-Scheduler-Internals.md` to reflect `PT30S` `ExecutionTimeLimit`.
- Updated `SECURITY.md` supported versions table to `1.3.x`.
- Added `cargo audit` job to `.github/workflows/security.yml`.

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
- Fixed `VBVMR_Login()` error handling to distinguish code `0` (success) from code `1` (Voicemeeter not launched) and negative error returns, invoking `VBVMR_Logout()` before unmapping library to prevent client resource leaks.
- Fixed XPath generation to preserve single quotes and apostrophes in device names via XPath 1.0 `concat()`, enforcing the minimum 2-argument arity required by W3C specifications.
- Fixed device name parsing to extract hardware adapters across all Windows operating system languages (e.g. German, French, Spanish, Japanese, Chinese) while preserving embedded trademarks (e.g. `Realtek(R) Audio`, `Intel(R) Display Audio`).
- Automatically enable the `Microsoft-Windows-Audio/Operational` event log channel during installation via `wevtutil.exe` to guarantee Event 65 records on clean Windows installations.
- Added `--disable-channel` flag to `uninstall` command to allow optional deactivation of the audio event channel.
- Added language-independent XML status inspection in `query_task_status()`, eliminating localized string scraping issues on international Windows editions.
- Added dynamic buffer reallocation for `ERROR_MORE_DATA` (234) and `REG_EXPAND_SZ` environment variable expansion via `ExpandEnvironmentStringsW` in registry discovery.
- Expanded registry discovery subkeys to cover Voicemeeter Standard, Banana, and Potato editions.
- Fixed exit code in `uninstall` command to return status code `1` on failure and combined `stdout`/`stderr` reporting so errors are never blank.
- Added check for Voicemeeter presence prior to sleeping in `restart` command to prevent idle blocking when Voicemeeter is offline.
- Added `--help` / `-h` handling across subcommands and prevented accidental task deletion when running `uninstall --help`.

### Changed
- Converted process snapshot matching in `is_voicemeeter_running()` to zero-allocation UTF-16 slice comparison against string literals.
- Added strict bounds checking for `--delay-ms` (`0 <= delay <= 30000`), option validation per subcommand, and support for `--user` and `--disable-channel`.
- Updated event filter to monitor both playback (`flow='0'`) and capture (`flow='1'`) endpoints, with `IgnoreNew` policy to debounce rapid events without restart storms.
- Synchronized Voicemeeter client cache with dirty polls before reading device parameters.
- Harmonized author email to `johnathangallagherusa@gmail.com` across all project files.
- Added multi-target matrix (`x86_64` and `i686`) and standalone executable publishing in release workflow.

## [1.1.1] - 2026-09-18

### Fixed
- Added a 150ms hold in `restart_audio_engine` before client drop to guarantee Voicemeeter's message loop consumes `Command.Restart` before shared memory is unmapped during logout.
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
