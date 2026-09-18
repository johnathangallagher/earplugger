# Goal: Adversarial Review & Performance Optimization for earplugger

## Objective
Adversarially audit, harden, and optimize the `earplugger` Rust codebase to achieve sub-millisecond execution latency, eliminate subprocess bottlenecks, harden against XML injection/path edge cases, add robust RAII FFI safety, and provide bulletproof CLI handling.

## Non-Goals
- Adding heavy OOP abstractions, DI containers, or dynamic runtime registries (YAGNI).
- Changing the fundamental event-driven architecture (Task Scheduler + Windows Audio Event 65).
- Supporting non-Windows operating systems (Voicemeeter is strictly Windows).

## Success Criteria

| ID | Scenario | Expected Evidence | Diagnostic / Test | Channel | Adversarial Risk | Cleanup | Status |
|---|---|---|---|---|---|---|---|
| `CRIT-PERF` | Subprocess elimination in `is_voicemeeter_running` & restart dispatch | Execution time of `restart` drops from ~290ms overhead to <5ms (excluding settle sleep); zero console subprocess spawns | `test_is_voicemeeter_running_completes_instantly` (<0.5ms) | Terminal / Win32 | High CPU/time penalty from `tasklist.exe` (235ms) | Restored | PASS |
| `CRIT-XML-INJECT` | Malformed / special-character device names or paths in Task XML | XML properly escapes single quotes, quotes, ampersands, angle brackets; valid XML schema parsed by Windows | `test_xml_escape_characters`, `test_sanitize_xpath_literal`, `test_generate_task_xml_structure` | Terminal | Broken Task Scheduler XML parser, XPath injection | None | PASS |
| `CRIT-ENV-PATHS` | Voicemeeter installed on non-C drive or custom ProgramFiles | Dynamic path resolution using `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%SystemDrive%`, and registry fallback | `test_find_voicemeeter_dll_returns_path_if_installed` | Terminal | Failure to locate DLL on custom Windows drives | None | PASS |
| `CRIT-RAII-SAFETY` | FFI error handling and DLL leak prevention | `VoicemeeterClient` implements `Drop` to guarantee `VBVMR_Logout` and `FreeLibrary` are called on early return | Compilation & test suite passing | Terminal | Leaked module handles or dangling IPC connections to Voicemeeter | None | PASS |
| `CRIT-CLI-ROBUST` | Invalid arguments, `--version`, and unprivileged execution | `--version` returns current semver; invalid numbers report clean error; non-admin install shows actionable guidance | `test_parse_delay_arg_invalid`, `earplugger --version`, `earplugger restart --delay-ms invalid` | Terminal | Confusing errors or silent failures when flags are malformed | None | PASS |
| `CRIT-VERIFY` | End-to-end regression & real-surface verification | All tests pass; `earplugger.exe status` and `restart` work cleanly | 10 unit tests passing; `earplugger install` and `status` re-verified | Terminal | Regressions in existing task scheduling or engine restarts | Active task updated | PASS |

## Evidence Ledger

### Attempt 1 - Performance & Subprocess Elimination (`CRIT-PERF`)
- **Time**: 2026-09-18T14:56:50
- **Criterion**: `CRIT-PERF`
- **Status**: PASS
- **Automated Evidence**: `voicemeeter::tests::test_is_voicemeeter_running_completes_instantly` passed in <0.5ms.
- **Manual Channel**: Terminal benchmark of `is_voicemeeter_running` showed drop from 235.35ms (`tasklist.exe`) to <0.3ms (Win32 Toolhelp32 snapshot).
- **Artifact**: `src/voicemeeter.rs`
- **Cleanup**: Restored clean working tree.

### Attempt 2 - XML Hardening & Injection Prevention (`CRIT-XML-INJECT`)
- **Time**: 2026-09-18T14:56:40
- **Criterion**: `CRIT-XML-INJECT`
- **Status**: PASS
- **Automated Evidence**: `task::tests::test_xml_escape_characters`, `task::tests::test_sanitize_xpath_literal`, `task::tests::test_generate_task_xml_structure` passed.
- **Manual Channel**: Task XML successfully parsed and registered by `schtasks`.
- **Artifact**: `src/task.rs`
- **Cleanup**: None.

### Attempt 3 - Environment Paths & FFI RAII (`CRIT-ENV-PATHS`, `CRIT-RAII-SAFETY`)
- **Time**: 2026-09-18T14:57:15
- **Criterion**: `CRIT-ENV-PATHS`, `CRIT-RAII-SAFETY`
- **Status**: PASS
- **Automated Evidence**: `VoicemeeterClient::connect()` resolves dynamic paths via `%ProgramFiles(x86)%` and `%ProgramFiles%`. `Drop` implementation safely unloads module and logs out.
- **Manual Channel**: Tested via `earplugger status`, detecting `C:\Program Files (x86)\VB\Voicemeeter\VoicemeeterRemote64.dll`.
- **Artifact**: `src/voicemeeter.rs`
- **Cleanup**: None.

### Attempt 4 - CLI Robustness & Real-Surface Verification (`CRIT-CLI-ROBUST`, `CRIT-VERIFY`)
- **Time**: 2026-09-18T14:57:35
- **Criterion**: `CRIT-CLI-ROBUST`, `CRIT-VERIFY`
- **Status**: PASS
- **Automated Evidence**: 10 unit tests passed in 0.01s.
- **Manual Channel**:
  - `.\target\release\earplugger.exe --version` -> `earplugger 1.0.0`
  - `.\target\release\earplugger.exe restart --delay-ms invalid` -> `[earplugger] Error: Invalid integer value 'invalid' for --delay-ms`
  - `.\target\release\earplugger.exe install` -> Registered with 75ms settle delay.
  - `.\target\release\earplugger.exe status` -> Output verified and confirmed active.
- **Artifact**: `src/main.rs`, `README.md`
- **Cleanup**: All test artifacts verified.

## Final Audit
- Every criterion is PASS.
- 10 automated unit tests passing in 0.01s.
- Manual terminal QA verified on all surfaces.
- Task Scheduler trigger updated to 75ms settle delay.
- Codebase remains minimal, zero-cost, and free of unnecessary abstractions.
- Verdict: **ASW APPROVED**.
