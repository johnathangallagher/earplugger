# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Relocated `install.bat` and `uninstall.bat` from repo root into `src/` (`src/install.bat` and `src/uninstall.bat`), keeping the repository root minimal and uncluttered.
- Updated `src/install.bat` and `src/uninstall.bat` binary resolution to dynamically locate `earplugger.exe` across release distributions, root directories, and cargo target build directories.
- Grouped `if exist` commands in parentheses in `src/install.bat` and `src/uninstall.bat` to prevent unconditional execution of subsequent `goto run_exe`.
- Hardened fallback argument parsing in `src/uninstall.bat` using iterative `shift` parsing.
- Extracted `strip_driver_prefix` to preserve friendly names during Voicemeeter A1 auto-detection, enabling dual-clause XPath matching in generated task triggers.
- Replaced permissive `!task_file.is_file()` checks in `query_task_status` and `uninstall_task` with explicit `io::ErrorKind::NotFound` matching.
- Updated `.github/workflows/release.yml` to package `src/install.bat` and `src/uninstall.bat` within `src/` in release archives.
- Optimized `.github/workflows/security.yml` dependency audit by installing prebuilt `cargo-audit` via `taiki-e/install-action@cargo-audit`, eliminating the ~3 minute compilation bottleneck from source.
- Streamlined `README.md` to provide direct, clean, and fluff-free setup and architectural documentation while preserving the project AI disclaimer banner.

## [1.3.0] - 2026-09-18

### Fixed

- Streamlined `install.bat` and `uninstall.bat` to forward command-line arguments `%*` directly to `earplugger.exe` with `DisableDelayedExpansion`, eliminating batch argument splitting, delayed expansion exclamation mark stripping, quote stripping syntax errors, and numeric validation bypasses.
- Explicitly rejected device names containing both single and double quotes in `parse_options` and `format_xpath_string_literal` with actionable error diagnostics, preventing dead Task Scheduler triggers since Windows Event Log XPath does not support escaped quotes or `concat()`.
- Maintained Task Scheduler `<MultipleInstancesPolicy>` as `IgnoreNew` to debounce composite USB device re-enumeration storms and prevent concurrent Voicemeeter restart race conditions.
- Omitted `<UserId>` when `--user` is unspecified, allowing Task Scheduler `<LogonType>InteractiveToken</LogonType>` to dynamically bind to whichever user is interactively logged in rather than the elevated Administrator account.
- Invoked Win32 `FreeConsole()` inside `handle_restart` when `--silent` is passed, eliminating console window flashing while ensuring interactive subcommands and argument validation retain console output without runtime panics.
- Added dual-clause XPath matching for parenthetical device friendly names (e.g. `(Data[@Name='DeviceName']='Speakers (RODE NT-USB)' or Data[@Name='DeviceName']='RODE NT-USB')`) using `clean_device_name`, guaranteeing trigger matching whether Windows MMDevAPI logs the endpoint friendly name or hardware adapter description in Event 65.
- Invoked `clean_device_name` during Voicemeeter A1 auto-detection in `handle_install`, correctly stripping driver prefixes and endpoint indices across all driver types.
- Hardened `parse_task_xml_status` to support attribute-bearing `<EventTrigger>` and `<Settings>` XML elements.
- Implemented `RegKeyGuard` RAII drop wrapper in `query_registry_uninstall_dir` and replaced `.exists()` with `.is_file()` to prevent directory preloading vulnerabilities.
- Made `query_task_status` and `uninstall_task` language-independent by inspecting `System32\Tasks` file existence and Win32 error codes.
- Fixed 32-bit `Sysnative` redirector bypass in `get_system32_path` by testing executable file existence (`sysnative_dir.join("cmd.exe").is_file()`) instead of directory status on the virtual alias.
- Added explicit case-insensitive wildcard trigger support via `--device "*"` / `--device "any"` / `--device "all"` / `--device "-"`, allowing users to deliberately bypass auto-detection when Voicemeeter is active.
- Hardened `uninstall.bat` fallback path when `earplugger.exe` is absent: safe argument iteration avoiding subshell pipe parsing, preserved `schtasks` error code, and checked task file existence.
- Hardened `install_task` temporary XML file allocation by binding `TempFileGuard` immediately upon file creation, preventing leaked temporary files in `%TEMP%` if `write_all` or `flush` fails.
- Hardened `uninstall_task` to treat missing task deletions as non-fatal, ensuring event channel disablement (`--disable-channel`) executes cleanly even if the task was already deleted.
- Re-initialized `entry.dwSize` before `Process32NextW` inside the Voicemeeter process enumeration loop per Win32 Toolhelp32 specifications.
- Removed ambiguous `"micro"` shorthand from `ENDPOINT_ROLES` and implemented exact phrase and word-boundary token matching, preventing false-positive stripping of device names starting with "micro" or brand names containing role words.
- Replaced `.to_str()` requirement on temporary XML file path with direct `&OsStr` argument passing to `schtasks.exe`, supporting arbitrary non-UTF-8 temporary directory paths.
- Hardened `parse_task_xml_status` to evaluate both `<Settings><Enabled>` and `<EventTrigger><Enabled>` blocks, detecting trigger-level disabled status.
- Added explicit top-level `permissions: contents: read` to `.github/workflows/ci.yml`.
- Converted MediaWiki-style links in `docs/wiki/Home.md` to standard Markdown relative links.
- Corrected XPath attribute quotes in `docs/wiki/Task-Scheduler-Internals.md` to match the exact single-quoted implementation.
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
- Delegated direct flag invocations (e.g. `earplugger --silent`) to the default `restart` command in `main()`.
- Separated release packaging from release publication in `.github/workflows/release.yml` with a downstream `publish` job, eliminating concurrent release publishing race conditions.
- Updated `docs/wiki/Task-Scheduler-Internals.md` to reflect `PT30S` `ExecutionTimeLimit`.
- Updated `SECURITY.md` supported versions table to `1.3.x`.

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
- Fixed XPath generation to preserve single quotes and apostrophes in device names via double-quoted XPath string literals.
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
