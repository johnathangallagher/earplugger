# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-09-18

### Fixed

- Fixed `query_task_status` to properly decode `schtasks /query /xml` UTF-16 LE output (BOM-detected). Previously the raw bytes were decoded as UTF-8 via `from_utf8_lossy`, producing garbage; the `is_enabled` field was always `true` regardless of actual task state.
- Fixed `<Enabled>` check in `query_task_status` to inspect only the `<Settings>` block, avoiding false positives from trigger-level `<Enabled>false</Enabled>` elements.
- Fixed `data_len` not being reset before the retry `RegQueryValueExW` call after `ERROR_MORE_DATA` (234). Previously the stale `data_len` from the first call was passed as the buffer capacity, which could cause the API to write beyond the resized buffer.
- Fixed `char_count` computation in registry read to clamp to `buf.len()`, preventing a potential panic if the registry value grows between the first and second calls (TOCTOU).
- Fixed `ExpandEnvironmentStringsW` to receive an explicit null-terminated copy of the logical string (`buf[..len]` + `\0`), not the raw full registry buffer. Previously the API read past the logical string end, relying on implicit zero-initialization.
- Fixed `ExpandEnvironmentStringsW` result handling to find the NUL explicitly in the output slice rather than trusting `exp_len - 1`; a buffer that exactly fills without a NUL would have included the terminator character in the decoded Rust `String`.
- Fixed `VBVMR_Login` error handling to call `VBVMR_Logout` unconditionally for all non-zero return codes, not only code `1`. Negative codes may partially initialize internal communication state; unconditional logout on failure is safe per VB-Audio SDK semantics.
- Fixed `parse_uninstall_string_dir` to use `as_bytes().windows(4).position(|w| w.eq_ignore_ascii_case(b".exe"))` for the `.exe` search, eliminating the `to_lowercase()` index reuse which is a latent panic vector for non-ASCII path characters.
- Fixed `clean_device_name` to use `rfind(" (")` instead of `find(" (")` for outermost parenthetical extraction. The previous `find` produced corrupted output for endpoint role names containing their own parenthetical groups (e.g. `"Kopfhörer (Dynamisch) (RODE NT-USB)"`).
- Fixed `clean_device_name` driver prefix detection to use an explicit allowlist (`WDM`, `MME`, `KS`, `ASIO`, `DirectSound`) instead of the prior `is_alphanumeric || '_'` heuristic, which incorrectly stripped prefixes like `"Focusrite"` or `"USB_Audio"`.
- Fixed `restart_audio_engine` dirty poll loop to break early when `is_parameters_dirty()` returns `0` (parameter consumed), rather than always sleeping the full 150ms.
- Fixed `get_system32_path` to retry `GetSystemDirectoryW` with the required buffer size when the initial 260-character buffer is too small, rather than silently falling back to `%SystemRoot%`.
- Fixed temp file path to use `.to_str()` with an explicit error return rather than `.to_string_lossy()`, preventing silent path corruption on Windows with non-UTF-8 `%TEMP%` paths (e.g. East Asian locale user profiles).
- Fixed non-silent `restart` mode to exit 0 when Voicemeeter closes during the settle delay, matching silent mode behavior. Previously this returned exit code 1, causing Task Scheduler to log spurious failures.
- Fixed `install.bat` argument passthrough to use an explicit allowlist parser instead of raw `%*`, eliminating a cmd.exe metacharacter injection vector.
- Reduced `<ExecutionTimeLimit>` from `PT1M` to `PT30S` in the Task Scheduler XML definition. The `IgnoreNew` policy suppresses re-triggering while a prior instance runs; a 30-second limit narrows the dead zone for rapid successive KVM switches.
- Annotated `eq_ignore_ascii_case_wide_str` with a `debug_assert` enforcing the ASCII-only precondition on the `ascii` argument.
- Refactored `std::mem::transmute` calls for `GetProcAddress` results to use `Option<fn>` intermediate type, the canonical Rust pattern for converting data pointers to function pointers.
- Removed the redundant `_login_fn` field from `VoicemeeterClient`; the login function pointer is not needed after the constructor returns.
- Added `cargo audit` as a parallel job in `.github/workflows/security.yml` to catch known CVE advisories in dependencies independent of CodeQL Rust beta coverage.

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
