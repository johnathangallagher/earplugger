# Security Policy

## Supported Versions

Security updates are provided for the latest minor release line:

| Version | Status |
| ------- | ------ |
| 1.2.x   | Maintained |
| < 1.2.0 | Unsupported |

---

## Reporting a Vulnerability

If you identify a security vulnerability in `earplugger` (such as issues related to Win32 FFI safety, scheduled task XML/XPath generation, or process permissions), please report it responsibly.

### How to Report
- Open a private advisory report through GitHub Security Advisories at https://github.com/johnathangallagher/earplugger/security/advisories/new
- Or email Johnathan Gallagher directly at `johnathangallagherusa@gmail.com` with the subject line `[SECURITY] earplugger Vulnerability Report`.

### What to Include
1. Clear description of the vulnerability and potential impact.
2. Steps to reproduce or proof-of-concept.
3. Windows version, build, and Voicemeeter edition.
4. Any suggested remediations or mitigations.

### Response Expectations
Reports are handled on a best-effort basis as time permits. There are no formal response timelines or resolution SLAs. Confirmed security fixes will be tagged and released via standard repository releases.

---

## Security Model & Threat Boundaries

1. **Least Privilege Execution:** The background scheduled task runs with `LeastPrivilege` under the interactive user logon token.
2. **Deterministic Library Resolution:** Dynamic library loading resolves DLLs exclusively from standard system and vendor paths using `LOAD_WITH_ALTERED_SEARCH_PATH`.
3. **Task Definition Sanitization:** Scheduled task definitions are escaped against XML entity injection and XPath 1.0 injection.
4. **Qualified System Utilities:** Administrative utilities (`schtasks.exe`, `wevtutil.exe`) are invoked via absolute paths rooted in `%SystemRoot%\System32`.
