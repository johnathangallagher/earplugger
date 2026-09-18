use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

type LoginFn = unsafe extern "system" fn() -> i32;
type LogoutFn = unsafe extern "system" fn() -> i32;
type SetParamFn = unsafe extern "system" fn(*const i8, f32) -> i32;
type GetParamStringWFn = unsafe extern "system" fn(*const i8, *mut u16) -> i32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const i8) -> *mut c_void;
    fn FreeLibrary(hLibModule: *mut c_void) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn find_voicemeeter_dll() -> Option<PathBuf> {
    let candidates = [
        r"C:\Program Files (x86)\VB\Voicemeeter\VoicemeeterRemote64.dll",
        r"C:\Program Files\VB\Voicemeeter\VoicemeeterRemote64.dll",
        r"C:\Program Files (x86)\VB\Voicemeeter\VoicemeeterRemote.dll",
    ];

    for c in &candidates {
        let p = Path::new(c);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

pub fn is_voicemeeter_running() -> bool {
    let procs = [
        "voicemeeter8x64",
        "voicemeeter8",
        "voicemeeterpro_x64",
        "voicemeeterpro",
        "voicemeeter_x64",
        "voicemeeter",
    ];
    let output = std::process::Command::new("tasklist")
        .arg("/NH")
        .arg("/FO")
        .arg("CSV")
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        for proc in procs {
            if text.contains(proc) {
                return true;
            }
        }
    }
    false
}

pub fn restart_audio_engine(delay_ms: u64) -> Result<(), String> {
    if delay_ms > 0 {
        sleep(Duration::from_millis(delay_ms));
    }

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

        if login_ptr.is_null() || logout_ptr.is_null() || set_param_ptr.is_null() {
            FreeLibrary(h_module);
            return Err("Failed to resolve Voicemeeter API entry points".to_string());
        }

        let login: LoginFn = std::mem::transmute(login_ptr);
        let logout: LogoutFn = std::mem::transmute(logout_ptr);
        let set_param: SetParamFn = std::mem::transmute(set_param_ptr);

        let res = login();
        if res < 0 {
            FreeLibrary(h_module);
            return Err(format!("VBVMR_Login returned error code: {}", res));
        }

        let cmd = b"Command.Restart\0";
        set_param(cmd.as_ptr() as *const i8, 1.0);
        sleep(Duration::from_millis(50));
        logout();

        FreeLibrary(h_module);
    }

    Ok(())
}

pub fn get_a1_device_name() -> Result<String, String> {
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
        let get_param_str_ptr = GetProcAddress(
            h_module,
            b"VBVMR_GetParameterStringW\0".as_ptr() as *const i8,
        );

        if login_ptr.is_null() || logout_ptr.is_null() || get_param_str_ptr.is_null() {
            FreeLibrary(h_module);
            return Err("Failed to resolve Voicemeeter API entry points".to_string());
        }

        let login: LoginFn = std::mem::transmute(login_ptr);
        let logout: LogoutFn = std::mem::transmute(logout_ptr);
        let get_param_str: GetParamStringWFn = std::mem::transmute(get_param_str_ptr);

        let res = login();
        if res < 0 {
            FreeLibrary(h_module);
            return Err(format!("VBVMR_Login returned error code: {}", res));
        }

        let mut buffer = [0u16; 512];
        let param_name = b"Bus[0].device.name\0";
        get_param_str(param_name.as_ptr() as *const i8, buffer.as_mut_ptr());

        logout();
        FreeLibrary(h_module);

        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let device_name = String::from_utf16_lossy(&buffer[..len]);
        Ok(device_name)
    }
}
