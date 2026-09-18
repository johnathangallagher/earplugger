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

const HKEY_LOCAL_MACHINE: *mut c_void = 0x80000002usize as *mut c_void;
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

fn query_registry_uninstall_dir() -> Option<PathBuf> {
    let subkeys = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\VB:Voicemeeter {173CE46D-8D5C-461C-B001-57753AB21EAE}",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\VB:Voicemeeter {173CE46D-8D5C-461C-B001-57753AB21EAE}",
    ];
    let val_name = to_wide_str("UninstallString");

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
                let mut data_len: u32 = 1024;
                let mut buf = vec![0u8; data_len as usize];
                let mut val_type: u32 = 0;
                let query_res = unsafe {
                    RegQueryValueExW(
                        hkey,
                        val_name.as_ptr(),
                        std::ptr::null_mut(),
                        &mut val_type,
                        buf.as_mut_ptr(),
                        &mut data_len,
                    )
                };
                unsafe {
                    RegCloseKey(hkey);
                }
                if query_res == 0 && data_len > 0 {
                    // UninstallString is typically "C:\Program Files (x86)\VB\Voicemeeter\voicemeeterprosetup.exe"
                    let u16_slice: &[u16] = unsafe {
                        std::slice::from_raw_parts(
                            buf.as_ptr() as *const u16,
                            (data_len / 2) as usize,
                        )
                    };
                    let len = u16_slice
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(u16_slice.len());
                    let path_str = String::from_utf16_lossy(&u16_slice[..len]);
                    let clean = path_str.trim_matches('"');
                    let exe_path = PathBuf::from(clean);
                    if let Some(parent) = exe_path.parent() {
                        let candidate = parent.join(DLL_NAME);
                        if candidate.exists() {
                            return Some(candidate);
                        }
                    }
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

#[inline]
fn eq_ignore_ascii_case_wide(a: &[u16], b: &[u16]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(&x, &y)| {
        let cx = if (b'A' as u16..=b'Z' as u16).contains(&x) {
            x + 32
        } else {
            x
        };
        let cy = if (b'A' as u16..=b'Z' as u16).contains(&y) {
            y + 32
        } else {
            y
        };
        cx == cy
    })
}

// UTF-16 representations of known Voicemeeter executable binaries for zero-allocation matching
const VM_EXES: &[&[u16]] = &[
    &[
        118, 111, 105, 99, 101, 109, 101, 101, 116, 101, 114, 56, 120, 54, 52, 46, 101, 120, 101,
    ], // voicemeeter8x64.exe
    &[
        118, 111, 105, 99, 101, 109, 101, 101, 116, 101, 114, 56, 46, 101, 120, 101,
    ], // voicemeeter8.exe
    &[
        118, 111, 105, 99, 101, 109, 101, 101, 116, 101, 114, 112, 114, 111, 95, 120, 54, 52, 46,
        101, 120, 101,
    ], // voicemeeterpro_x64.exe
    &[
        118, 111, 105, 99, 101, 109, 101, 101, 116, 101, 114, 112, 114, 111, 46, 101, 120, 101,
    ], // voicemeeterpro.exe
    &[
        118, 111, 105, 99, 101, 109, 101, 101, 116, 101, 114, 95, 120, 54, 52, 46, 101, 120, 101,
    ], // voicemeeter_x64.exe
    &[
        118, 111, 105, 99, 101, 109, 101, 101, 116, 101, 114, 46, 101, 120, 101,
    ], // voicemeeter.exe
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

                for &target in VM_EXES {
                    if eq_ignore_ascii_case_wide(exe_slice, target) {
                        return true;
                    }
                }

                entry.dwSize = std::mem::size_of::<ProcessEntry32W>() as u32;
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
    _login_fn: LoginFn,
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

        unsafe {
            let h_module = LoadLibraryExW(
                wide_path.as_ptr(),
                std::ptr::null_mut(),
                LOAD_WITH_ALTERED_SEARCH_PATH,
            );
            if h_module.is_null() {
                return Err(format!("Failed to load {}", dll_path.display()));
            }

            let login_ptr = GetProcAddress(h_module, c"VBVMR_Login".as_ptr());
            let logout_ptr = GetProcAddress(h_module, c"VBVMR_Logout".as_ptr());
            let set_param_ptr = GetProcAddress(h_module, c"VBVMR_SetParameterFloat".as_ptr());
            let get_param_str_ptr = GetProcAddress(h_module, c"VBVMR_GetParameterStringW".as_ptr());
            let is_dirty_ptr = GetProcAddress(h_module, c"VBVMR_IsParametersDirty".as_ptr());

            if login_ptr.is_null()
                || logout_ptr.is_null()
                || set_param_ptr.is_null()
                || get_param_str_ptr.is_null()
            {
                FreeLibrary(h_module);
                return Err("Failed to resolve Voicemeeter API entry points".to_string());
            }

            let login: LoginFn = std::mem::transmute(login_ptr);
            let logout: LogoutFn = std::mem::transmute(logout_ptr);
            let set_param: SetParamFn = std::mem::transmute(set_param_ptr);
            let get_param_str: GetParamStringWFn = std::mem::transmute(get_param_str_ptr);
            let is_dirty: Option<IsDirtyFn> = if !is_dirty_ptr.is_null() {
                Some(std::mem::transmute::<*mut c_void, IsDirtyFn>(is_dirty_ptr))
            } else {
                None
            };

            let res = login();
            if res != 0 {
                FreeLibrary(h_module);
                return Err(if res == 1 {
                    "Voicemeeter is not running".to_string()
                } else {
                    format!("VBVMR_Login failed with error code: {}", res)
                });
            }

            Ok(Self {
                h_module,
                logged_in: true,
                _login_fn: login,
                logout_fn: logout,
                set_param_fn: set_param,
                get_param_str_fn: get_param_str,
                is_dirty_fn: is_dirty,
            })
        }
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

    // Poll parameters dirty status with bounded sleep so Voicemeeter consumes Command.Restart
    for _ in 0..10 {
        sleep(Duration::from_millis(15));
        let _ = client.is_parameters_dirty();
    }
    Ok(())
}

pub fn get_a1_device_name() -> Result<String, String> {
    let client = VoicemeeterClient::connect()?;
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
    fn test_is_voicemeeter_running_does_not_panic() {
        let running = is_voicemeeter_running();
        // Simply assert the function runs cleanly and produces a boolean result
        let _ = running;
    }

    #[test]
    fn test_ascii_case_insensitive_wide_matching() {
        let a = to_wide_str("VoiceMeeterPro.exe");
        let b = to_wide_str("voicemeeterpro.exe");
        assert!(eq_ignore_ascii_case_wide(
            &a[..a.len() - 1],
            &b[..b.len() - 1]
        ));

        let c = to_wide_str("other_process.exe");
        assert!(!eq_ignore_ascii_case_wide(
            &a[..a.len() - 1],
            &c[..c.len() - 1]
        ));
    }
}
