use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

type LoginFn = unsafe extern "system" fn() -> i32;
type LogoutFn = unsafe extern "system" fn() -> i32;
type SetParamFn = unsafe extern "system" fn(*const i8, f32) -> i32;
type GetParamStringWFn = unsafe extern "system" fn(*const i8, *mut u16) -> i32;

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

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const i8) -> *mut c_void;
    fn FreeLibrary(hLibModule: *mut c_void) -> i32;
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut c_void;
    fn Process32FirstW(hSnapshot: *mut c_void, lppe: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(hSnapshot: *mut c_void, lppe: *mut ProcessEntry32W) -> i32;
    fn CloseHandle(hObject: *mut c_void) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn find_voicemeeter_dll() -> Option<PathBuf> {
    let mut search_paths = Vec::new();

    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        let base = Path::new(&pf86).join("VB").join("Voicemeeter");
        search_paths.push(base.join("VoicemeeterRemote64.dll"));
        search_paths.push(base.join("VoicemeeterRemote.dll"));
    }

    if let Ok(pf) = std::env::var("ProgramW6432") {
        let base = Path::new(&pf).join("VB").join("Voicemeeter");
        search_paths.push(base.join("VoicemeeterRemote64.dll"));
    } else if let Ok(pf) = std::env::var("ProgramFiles") {
        let base = Path::new(&pf).join("VB").join("Voicemeeter");
        search_paths.push(base.join("VoicemeeterRemote64.dll"));
        search_paths.push(base.join("VoicemeeterRemote.dll"));
    }

    if let Ok(drive) = std::env::var("SystemDrive") {
        let base = PathBuf::from(format!(r"{}\Program Files (x86)\VB\Voicemeeter", drive));
        search_paths.push(base.join("VoicemeeterRemote64.dll"));
    }

    search_paths.push(PathBuf::from(r"C:\Program Files (x86)\VB\Voicemeeter\VoicemeeterRemote64.dll"));
    search_paths.push(PathBuf::from(r"C:\Program Files\VB\Voicemeeter\VoicemeeterRemote64.dll"));
    search_paths.push(PathBuf::from(r"C:\Program Files (x86)\VB\Voicemeeter\VoicemeeterRemote.dll"));

    for p in search_paths {
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub fn is_voicemeeter_running() -> bool {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE || snapshot.is_null() {
            return false;
        }

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
                let exe_name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();

                let matches = matches!(
                    exe_name.as_str(),
                    "voicemeeter8x64.exe"
                        | "voicemeeter8.exe"
                        | "voicemeeterpro_x64.exe"
                        | "voicemeeterpro.exe"
                        | "voicemeeter_x64.exe"
                        | "voicemeeter.exe"
                );

                if matches {
                    CloseHandle(snapshot);
                    return true;
                }

                entry.dwSize = std::mem::size_of::<ProcessEntry32W>() as u32;
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
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
}

impl VoicemeeterClient {
    pub fn connect() -> Result<Self, String> {
        let dll_path = find_voicemeeter_dll().ok_or_else(|| {
            "VoicemeeterRemote64.dll not found in standard installation directories".to_string()
        })?;

        let wide_path = to_wide(&dll_path.to_string_lossy());

        unsafe {
            let h_module = LoadLibraryW(wide_path.as_ptr());
            if h_module.is_null() {
                return Err(format!("Failed to load {}", dll_path.display()));
            }

            let login_ptr = GetProcAddress(h_module, b"VBVMR_Login\0".as_ptr() as *const i8);
            let logout_ptr = GetProcAddress(h_module, b"VBVMR_Logout\0".as_ptr() as *const i8);
            let set_param_ptr =
                GetProcAddress(h_module, b"VBVMR_SetParameterFloat\0".as_ptr() as *const i8);
            let get_param_str_ptr = GetProcAddress(
                h_module,
                b"VBVMR_GetParameterStringW\0".as_ptr() as *const i8,
            );

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

            let res = login();
            if res < 0 {
                FreeLibrary(h_module);
                return Err(format!("VBVMR_Login returned error code: {}", res));
            }

            Ok(Self {
                h_module,
                logged_in: true,
                _login_fn: login,
                logout_fn: logout,
                set_param_fn: set_param,
                get_param_str_fn: get_param_str,
            })
        }
    }

    pub fn set_parameter_float(&self, param: &[u8], val: f32) -> Result<(), String> {
        let res = unsafe { (self.set_param_fn)(param.as_ptr() as *const i8, val) };
        if res < 0 {
            return Err(format!("SetParameterFloat failed with code: {}", res));
        }
        Ok(())
    }

    pub fn get_parameter_string_w(&self, param: &[u8]) -> Result<String, String> {
        let mut buffer = [0u16; 512];
        let res = unsafe {
            (self.get_param_str_fn)(param.as_ptr() as *const i8, buffer.as_mut_ptr())
        };
        if res < 0 {
            return Err(format!("GetParameterStringW failed with code: {}", res));
        }
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        Ok(String::from_utf16_lossy(&buffer[..len]))
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
    client.set_parameter_float(b"Command.Restart\0", 1.0)?;
    Ok(())
}

pub fn get_a1_device_name() -> Result<String, String> {
    let client = VoicemeeterClient::connect()?;
    client.get_parameter_string_w(b"Bus[0].device.name\0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_voicemeeter_dll_returns_path_if_installed() {
        if let Some(path) = find_voicemeeter_dll() {
            assert!(path.exists());
            assert!(path.to_string_lossy().ends_with(".dll"));
        }
    }

    #[test]
    fn test_is_voicemeeter_running_completes_instantly() {
        let start = std::time::Instant::now();
        let _ = is_voicemeeter_running();
        let elapsed = start.elapsed();
        // Native toolhelp snapshot must execute in under 20ms (typically <1ms)
        // compared to 235ms with tasklist.exe
        assert!(
            elapsed < Duration::from_millis(20),
            "Process detection took too long: {:?}",
            elapsed
        );
    }
}
