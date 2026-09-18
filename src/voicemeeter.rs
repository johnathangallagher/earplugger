use std::ffi::{CStr, c_void};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

#[cfg(target_pointer_width = "64")]
const DLL_NAME: &str = "VoicemeeterRemote64.dll";
#[cfg(target_pointer_width = "32")]
const DLL_NAME: &str = "VoicemeeterRemote.dll";

type LoginFn = unsafe extern "system" fn() -> i32;
type LogoutFn = unsafe extern "system" fn() -> i32;
type SetParamFn = unsafe extern "system" fn(*const i8, f32) -> i32;
type GetParamStringWFn = unsafe extern "system" fn(*const i8, *mut u16, i32) -> i32;
type IsDirtyFn = unsafe extern "system" fn() -> i32;

#[repr(C)]
#[allow(non_snake_case)]
struct ProcessEntry32W {
    dwSize: u32,
    cntUsage: u32,
    th32ProcessID: u32,
    th32DefaultHeapID: usize,
    th32ModuleID: u32,
    cntThreads: u32,
    th32ParentProcessID: u32,
    pcPriClassBase: i32,
    dwFlags: u32,
    szExeFile: [u16; 260],
}

const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x00000008;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(lpLibFileName: *const u16, hFile: *mut c_void, dwFlags: u32) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const i8) -> *mut c_void;
    fn FreeLibrary(hLibModule: *mut c_void) -> i32;
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut c_void;
    fn Process32FirstW(hSnapshot: *mut c_void, lppe: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(hSnapshot: *mut c_void, lppe: *mut ProcessEntry32W) -> i32;
    fn CloseHandle(hObject: *mut c_void) -> i32;
    fn ExpandEnvironmentStringsW(lpSrc: *const u16, lpDst: *mut u16, nSize: u32) -> u32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        hKey: *mut c_void,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut *mut c_void,
    ) -> i32;
    fn RegQueryValueExW(
        hKey: *mut c_void,
        lpValueName: *const u16,
        lpReserved: *mut u32,
        lpType: *mut u32,
        lpData: *mut u8,
        lpcbData: *mut u32,
    ) -> i32;
    fn RegCloseKey(hKey: *mut c_void) -> i32;
}

// Predefined keys in Windows SDK are sign-extended 32-bit values: ((HKEY)(ULONG_PTR)((LONG)0x80000002))
// On 64-bit Windows, (LONG)0x80000002 (-2147483646) sign-extends to 0xFFFFFFFF80000002.
const HKEY_LOCAL_MACHINE: *mut c_void = (-2147483646isize) as *mut c_void;
const KEY_READ: u32 = 0x20019;
const KEY_WOW64_32KEY: u32 = 0x0200;

struct SnapshotGuard(*mut c_void);

impl Drop for SnapshotGuard {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

fn to_wide_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn parse_uninstall_string_dir(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    let exe_str = if let Some(rest) = trimmed.strip_prefix('"') {
        if let Some(end_quote) = rest.find('"') {
            &rest[..end_quote]
        } else {
            trimmed.trim_matches('"')
        }
    } else if let Some(exe_end) = trimmed
        // Use ASCII-safe case-insensitive search to avoid to_lowercase() byte-length mismatch
        // on non-ASCII path characters (e.g. Ä, İ) which could cause incorrect slice indices.
        .as_bytes()
        .windows(4)
        .position(|w| w.eq_ignore_ascii_case(b".exe"))
    {
        &trimmed[..exe_end + 4]
    } else if let Some(space_idx) = trimmed.find(' ') {
        &trimmed[..space_idx]
    } else {
        trimmed
    };

    let exe_path = PathBuf::from(exe_str);
    exe_path.parent().map(|p| p.to_path_buf())
}

fn query_registry_uninstall_dir() -> Option<PathBuf> {
    let subkeys = [
        // Voicemeeter Banana
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\VB:Voicemeeter {173CE46D-8D5C-461C-B001-57753AB21EAE}",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\VB:Voicemeeter {173CE46D-8D5C-461C-B001-57753AB21EAE}",
        // Voicemeeter Standard
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\VB:Voicemeeter {17359A74-1236-5467}",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\VB:Voicemeeter {17359A74-1236-5467}",
        // Voicemeeter Potato
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\VB:VoicemeeterPotato {173CE46D-8D5C-461C-B001-57753AB21EAE}",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\VB:VoicemeeterPotato {173CE46D-8D5C-461C-B001-57753AB21EAE}",
    ];
    let val_names = [
        to_wide_str("InstallLocation"),
        to_wide_str("UninstallString"),
    ];

    for subkey in subkeys {
        let subkey_wide = to_wide_str(subkey);
        for flags in [KEY_READ, KEY_READ | KEY_WOW64_32KEY] {
            let mut hkey: *mut c_void = std::ptr::null_mut();
            let status = unsafe {
                RegOpenKeyExW(
                    HKEY_LOCAL_MACHINE,
                    subkey_wide.as_ptr(),
                    0,
                    flags,
                    &mut hkey,
                )
            };
            if status == 0 && !hkey.is_null() {
                for (val_idx, val_name) in val_names.iter().enumerate() {
                    // Use Vec<u16> to ensure proper 2-byte alignment for wide string deserialization
                    let mut buf = vec![0u16; 512];
                    let mut data_len = (buf.len() * std::mem::size_of::<u16>()) as u32;
                    let mut val_type: u32 = 0;
                    let mut query_res = unsafe {
                        RegQueryValueExW(
                            hkey,
                            val_name.as_ptr(),
                            std::ptr::null_mut(),
                            &mut val_type,
                            buf.as_mut_ptr() as *mut u8,
                            &mut data_len,
                        )
                    };

                    // Handle ERROR_MORE_DATA (234) dynamically if the registry path exceeds initial buffer.
                    // data_len must be reset to the new buffer capacity before the retry — RegQueryValueExW
                    // uses it as an in/out parameter.
                    if query_res == 234 && data_len > 0 {
                        let required_u16s = (data_len as usize).div_ceil(2);
                        buf.resize(required_u16s, 0);
                        // Reset data_len to the actual new buffer capacity in bytes before retry.
                        data_len = (buf.len() * std::mem::size_of::<u16>()) as u32;
                        query_res = unsafe {
                            RegQueryValueExW(
                                hkey,
                                val_name.as_ptr(),
                                std::ptr::null_mut(),
                                &mut val_type,
                                buf.as_mut_ptr() as *mut u8,
                                &mut data_len,
                            )
                        };
                    }

                    // Check success, valid string types (REG_SZ = 1, REG_EXPAND_SZ = 2), and non-empty length
                    if query_res == 0 && (val_type == 1 || val_type == 2) && data_len >= 2 {
                        // data_len from RegQueryValueExW is the byte count including the NUL terminator.
                        // Clamp char_count to buf.len() to guard against a TOCTOU where the registry value
                        // grows between the first (ERROR_MORE_DATA) and second calls.
                        let char_count = ((data_len as usize) / 2).min(buf.len());
                        let u16_slice = &buf[..char_count];
                        let len = u16_slice
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(u16_slice.len());

                        // If REG_EXPAND_SZ (2), expand environment variables like %ProgramFiles%
                        let resolved_str = if val_type == 2 {
                            // Build an explicit null-terminated copy of just the logical string to
                            // pass to ExpandEnvironmentStringsW. Passing the raw buf pointer would
                            // rely on zero-initialization past the string end; this is explicit.
                            let mut input: Vec<u16> = buf[..len].to_vec();
                            input.push(0);

                            let mut exp_buf = vec![0u16; 1024];
                            let exp_len = unsafe {
                                ExpandEnvironmentStringsW(
                                    input.as_ptr(),
                                    exp_buf.as_mut_ptr(),
                                    exp_buf.len() as u32,
                                )
                            };
                            // exp_len includes the NUL terminator. A return of 0 means API failure.
                            // A return > exp_buf.len() means output was truncated.
                            if exp_len > 0 && (exp_len as usize) <= exp_buf.len() {
                                // exp_len includes the NUL; find it explicitly rather than trusting the count.
                                let exp_end = exp_buf[..exp_len as usize]
                                    .iter()
                                    .position(|&c| c == 0)
                                    .unwrap_or(exp_len as usize - 1);
                                String::from_utf16_lossy(&exp_buf[..exp_end])
                            } else {
                                // Expansion failed or truncated — fall back to the unexpanded string.
                                String::from_utf16_lossy(&u16_slice[..len])
                            }
                        } else {
                            String::from_utf16_lossy(&u16_slice[..len])
                        };

                        let candidate_dir = if val_idx == 0 {
                            // InstallLocation is directly the directory path
                            let trimmed = resolved_str.trim().trim_matches('"');
                            if !trimmed.is_empty() {
                                Some(PathBuf::from(trimmed))
                            } else {
                                None
                            }
                        } else {
                            // UninstallString contains an executable path, potentially with arguments
                            parse_uninstall_string_dir(&resolved_str)
                        };

                        if let Some(dir) = candidate_dir {
                            let candidate_dll = dir.join(DLL_NAME);
                            if candidate_dll.exists() {
                                unsafe {
                                    RegCloseKey(hkey);
                                }
                                return Some(candidate_dll);
                            }
                        }
                    }
                }
                unsafe {
                    RegCloseKey(hkey);
                }
            }
        }
    }
    None
}

pub fn find_voicemeeter_dll() -> Option<PathBuf> {
    if let Some(reg_path) = query_registry_uninstall_dir() {
        return Some(reg_path);
    }

    let mut search_paths = Vec::new();

    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        let base = Path::new(&pf86).join("VB").join("Voicemeeter");
        search_paths.push(base.join(DLL_NAME));
    }

    if let Ok(pf) = std::env::var("ProgramW6432") {
        let base = Path::new(&pf).join("VB").join("Voicemeeter");
        search_paths.push(base.join(DLL_NAME));
    } else if let Ok(pf) = std::env::var("ProgramFiles") {
        let base = Path::new(&pf).join("VB").join("Voicemeeter");
        search_paths.push(base.join(DLL_NAME));
    }

    if let Ok(drive) = std::env::var("SystemDrive") {
        let base = PathBuf::from(format!(r"{}\Program Files (x86)\VB\Voicemeeter", drive));
        search_paths.push(base.join(DLL_NAME));
        let base_64 = PathBuf::from(format!(r"{}\Program Files\VB\Voicemeeter", drive));
        search_paths.push(base_64.join(DLL_NAME));
    }

    search_paths.push(PathBuf::from(format!(
        r"C:\Program Files (x86)\VB\Voicemeeter\{}",
        DLL_NAME
    )));
    search_paths.push(PathBuf::from(format!(
        r"C:\Program Files\VB\Voicemeeter\{}",
        DLL_NAME
    )));

    search_paths.into_iter().find(|p| p.exists())
}

// eq_ignore_ascii_case_wide_str compares a NUL-excluded wide string slice against an ASCII &str.
// PRECONDITION: `ascii` must be pure ASCII (all bytes 0x00–0x7F). The function is documented as
// ASCII-only: Voicemeeter process names are all ASCII, so this holds for all callers.
#[inline]
pub fn eq_ignore_ascii_case_wide_str(wide: &[u16], ascii: &str) -> bool {
    debug_assert!(
        ascii.is_ascii(),
        "eq_ignore_ascii_case_wide_str: ascii argument must be pure ASCII"
    );
    if wide.len() != ascii.len() {
        return false;
    }
    wide.iter().zip(ascii.bytes()).all(|(&w, b)| {
        let cw = if (b'A' as u16..=b'Z' as u16).contains(&w) {
            w + 32
        } else {
            w
        };
        let cb = if b.is_ascii_uppercase() {
            (b + 32) as u16
        } else {
            b as u16
        };
        cw == cb
    })
}

// Known Voicemeeter executable binary names for zero-allocation process matching
pub const VM_EXE_NAMES: &[&str] = &[
    "voicemeeter8x64.exe",
    "voicemeeter8.exe",
    "voicemeeterpro_x64.exe",
    "voicemeeterpro.exe",
    "voicemeeter_x64.exe",
    "voicemeeter.exe",
];

pub fn is_voicemeeter_running() -> bool {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE || snapshot.is_null() {
            return false;
        }
        let _guard = SnapshotGuard(snapshot);

        let mut entry = ProcessEntry32W {
            dwSize: std::mem::size_of::<ProcessEntry32W>() as u32,
            cntUsage: 0,
            th32ProcessID: 0,
            th32DefaultHeapID: 0,
            th32ModuleID: 0,
            cntThreads: 0,
            th32ParentProcessID: 0,
            pcPriClassBase: 0,
            dwFlags: 0,
            szExeFile: [0; 260],
        };

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe_slice = &entry.szExeFile[..len];

                for &target in VM_EXE_NAMES {
                    if eq_ignore_ascii_case_wide_str(exe_slice, target) {
                        return true;
                    }
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
    }
    false
}

pub struct VoicemeeterClient {
    h_module: *mut c_void,
    logged_in: bool,
    logout_fn: LogoutFn,
    set_param_fn: SetParamFn,
    get_param_str_fn: GetParamStringWFn,
    is_dirty_fn: Option<IsDirtyFn>,
}

impl VoicemeeterClient {
    pub fn connect() -> Result<Self, String> {
        let dll_path = find_voicemeeter_dll().ok_or_else(|| {
            format!(
                "{} not found in standard installation directories",
                DLL_NAME
            )
        })?;

        let mut wide_path: Vec<u16> = dll_path.as_os_str().encode_wide().collect();
        wide_path.push(0);

        let h_module = unsafe {
            LoadLibraryExW(
                wide_path.as_ptr(),
                std::ptr::null_mut(),
                LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        };
        if h_module.is_null() {
            return Err(format!("Failed to load {}", dll_path.display()));
        }

        let (login_ptr, logout_ptr, set_param_ptr, get_param_str_ptr, is_dirty_ptr) = unsafe {
            (
                GetProcAddress(h_module, c"VBVMR_Login".as_ptr()),
                GetProcAddress(h_module, c"VBVMR_Logout".as_ptr()),
                GetProcAddress(h_module, c"VBVMR_SetParameterFloat".as_ptr()),
                GetProcAddress(h_module, c"VBVMR_GetParameterStringW".as_ptr()),
                GetProcAddress(h_module, c"VBVMR_IsParametersDirty".as_ptr()),
            )
        };

        if login_ptr.is_null()
            || logout_ptr.is_null()
            || set_param_ptr.is_null()
            || get_param_str_ptr.is_null()
        {
            unsafe {
                FreeLibrary(h_module);
            }
            return Err("Failed to resolve Voicemeeter API entry points".to_string());
        }

        // Transmute via Option<fn> — the canonical Rust pattern for converting a data pointer
        // (returned by GetProcAddress) to a function pointer without UB. The null check above
        // guarantees the unwrap() cannot panic.
        let login: LoginFn =
            unsafe { std::mem::transmute::<*mut c_void, Option<LoginFn>>(login_ptr) }
                .expect("login_ptr null despite null check");
        let logout: LogoutFn =
            unsafe { std::mem::transmute::<*mut c_void, Option<LogoutFn>>(logout_ptr) }
                .expect("logout_ptr null despite null check");
        let set_param: SetParamFn =
            unsafe { std::mem::transmute::<*mut c_void, Option<SetParamFn>>(set_param_ptr) }
                .expect("set_param_ptr null despite null check");
        let get_param_str: GetParamStringWFn = unsafe {
            std::mem::transmute::<*mut c_void, Option<GetParamStringWFn>>(get_param_str_ptr)
        }
        .expect("get_param_str_ptr null despite null check");
        let is_dirty: Option<IsDirtyFn> = if !is_dirty_ptr.is_null() {
            unsafe { std::mem::transmute::<*mut c_void, Option<IsDirtyFn>>(is_dirty_ptr) }
        } else {
            None
        };

        let res = unsafe { login() };
        if res != 0 {
            // Call VBVMR_Logout for all non-success return codes to tear down any partially
            // initialized communication primitives. VBVMR_Login docs confirm codes 0 and 1 both
            // succeed in initializing the client; negative codes (-1, -2) may or may not initialize
            // state depending on the SDK version. Calling logout unconditionally on failure is safe
            // per the VB-Audio SDK: logout is a no-op if login never initialized state.
            unsafe {
                logout();
                FreeLibrary(h_module);
            }
            return Err(if res == 1 {
                "Voicemeeter is not running".to_string()
            } else {
                format!("VBVMR_Login failed with error code: {}", res)
            });
        }

        Ok(Self {
            h_module,
            logged_in: true,
            logout_fn: logout,
            set_param_fn: set_param,
            get_param_str_fn: get_param_str,
            is_dirty_fn: is_dirty,
        })
    }

    pub fn set_parameter_float(&self, param: &CStr, val: f32) -> Result<(), String> {
        let res = unsafe { (self.set_param_fn)(param.as_ptr(), val) };
        if res < 0 {
            return Err(format!("SetParameterFloat failed with code: {}", res));
        }
        Ok(())
    }

    pub fn get_parameter_string_w(&self, param: &CStr) -> Result<String, String> {
        let mut buffer = [0u16; 512];
        let res = unsafe {
            (self.get_param_str_fn)(param.as_ptr(), buffer.as_mut_ptr(), buffer.len() as i32)
        };
        if res < 0 {
            return Err(format!("GetParameterStringW failed with code: {}", res));
        }
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        Ok(String::from_utf16_lossy(&buffer[..len]))
    }

    pub fn is_parameters_dirty(&self) -> i32 {
        if let Some(is_dirty) = self.is_dirty_fn {
            unsafe { is_dirty() }
        } else {
            0
        }
    }
}

impl Drop for VoicemeeterClient {
    fn drop(&mut self) {
        unsafe {
            if self.logged_in {
                (self.logout_fn)();
                self.logged_in = false;
            }
            if !self.h_module.is_null() {
                FreeLibrary(self.h_module);
                self.h_module = std::ptr::null_mut();
            }
        }
    }
}

pub fn restart_audio_engine(delay_ms: u64) -> Result<(), String> {
    if delay_ms > 0 {
        sleep(Duration::from_millis(delay_ms));
    }

    let client = VoicemeeterClient::connect()?;
    client.set_parameter_float(c"Command.Restart", 1.0)?;

    // Hold client connection for 150ms while polling dirty status so Voicemeeter's
    // message loop consumes Command.Restart before shared memory is unmapped during logout.
    // Note: Command.Restart is a write-only trigger parameter that does not clear or set
    // is_parameters_dirty(); the full 150ms hold is required by VB-Audio SDK shared memory specs.
    for _ in 0..10 {
        sleep(Duration::from_millis(15));
        let _ = client.is_parameters_dirty();
    }
    Ok(())
}

pub fn get_a1_device_name() -> Result<String, String> {
    let client = VoicemeeterClient::connect()?;
    // Synchronize client cache before querying parameter
    for _ in 0..3 {
        sleep(Duration::from_millis(15));
        let _ = client.is_parameters_dirty();
    }
    client.get_parameter_string_w(c"Bus[0].device.name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_voicemeeter_dll_returns_path_if_installed() {
        if let Some(path) = find_voicemeeter_dll() {
            assert!(path.exists());
            assert!(path.to_string_lossy().ends_with(".dll"));
            #[cfg(target_pointer_width = "64")]
            assert!(path.to_string_lossy().ends_with("VoicemeeterRemote64.dll"));
            #[cfg(target_pointer_width = "32")]
            assert!(path.to_string_lossy().ends_with("VoicemeeterRemote.dll"));
        }
    }

    #[test]
    fn test_hkey_local_machine_sign_extension() {
        // Assert that HKEY_LOCAL_MACHINE is properly sign-extended to 0xFFFFFFFF80000002 on 64-bit targets
        #[cfg(target_pointer_width = "64")]
        assert_eq!(HKEY_LOCAL_MACHINE as usize, 0xFFFFFFFF80000002usize);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(HKEY_LOCAL_MACHINE as usize, 0x80000002usize);
    }

    #[test]
    fn test_is_voicemeeter_running_snapshot_execution() {
        // Verify that is_voicemeeter_running executes deterministic snapshot enumeration
        let r1 = is_voicemeeter_running();
        let r2 = is_voicemeeter_running();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_ascii_case_insensitive_wide_matching() {
        let wide = to_wide_str("VoicemeeterPro_x64.exe");
        assert!(eq_ignore_ascii_case_wide_str(
            &wide[..wide.len() - 1],
            "voicemeeterpro_x64.exe"
        ));

        let other = to_wide_str("chrome.exe");
        assert!(!eq_ignore_ascii_case_wide_str(
            &other[..other.len() - 1],
            "voicemeeterpro_x64.exe"
        ));
    }

    #[test]
    fn test_all_known_voicemeeter_binaries_match() {
        for &expected_name in VM_EXE_NAMES {
            let wide = to_wide_str(expected_name);
            let slice = &wide[..wide.len() - 1];
            let matched = VM_EXE_NAMES
                .iter()
                .any(|&target| eq_ignore_ascii_case_wide_str(slice, target));
            assert!(matched, "Failed to match binary name: {}", expected_name);

            // Also test uppercase variant
            let upper = expected_name.to_uppercase();
            let wide_upper = to_wide_str(&upper);
            let slice_upper = &wide_upper[..wide_upper.len() - 1];
            let matched_upper = VM_EXE_NAMES
                .iter()
                .any(|&target| eq_ignore_ascii_case_wide_str(slice_upper, target));
            assert!(
                matched_upper,
                "Failed to match uppercase binary name: {}",
                upper
            );
        }
    }

    #[test]
    fn test_parse_uninstall_string_dir() {
        let quoted = r#""C:\Program Files\VB\Voicemeeter\uninstall.exe" -silent"#;
        assert_eq!(
            parse_uninstall_string_dir(quoted),
            Some(PathBuf::from(r"C:\Program Files\VB\Voicemeeter"))
        );

        let unquoted = r#"C:\Program Files\VB\Voicemeeter\uninstall.exe /all"#;
        assert_eq!(
            parse_uninstall_string_dir(unquoted),
            Some(PathBuf::from(r"C:\Program Files\VB\Voicemeeter"))
        );

        let simple = r#"C:\Tools\Voicemeeter\unins000.exe"#;
        assert_eq!(
            parse_uninstall_string_dir(simple),
            Some(PathBuf::from(r"C:\Tools\Voicemeeter"))
        );
    }
}
