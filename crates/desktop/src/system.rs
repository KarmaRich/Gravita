//! 桌面端系统集成模块：Windows 纯原生 Win32 通知与注册表开机自启动管理（零外部 Shell / PowerShell 进程调用，彻底杜绝安全软件拦截）

use std::path::Path;
use std::sync::{Mutex, OnceLock};

const AUTOSTART_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const AUTOSTART_VALUE_NAME: &str = "IDM-Aria2-RS";
const WINDOW_TITLE: &str = "IDM-Aria2-RS 下载器";

/// 全局通知互斥锁：防止多个下载任务同时完成时，并发线程争抢同一个 Win32 `u_id` 导致气泡被提前删除
static NOTIFY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// 异步触发 Windows 原生通知（直接调用 Win32 `Shell_NotifyIconW` + `MessageBeep`，绝不拉起 PowerShell 或 cmd 子进程）
pub fn send_system_notification(title: impl Into<String>, body: impl Into<String>) {
    let title = title.into();
    let body = body.into();

    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            let lock = NOTIFY_LOCK.get_or_init(|| Mutex::new(()));
            let _guard = lock.lock().ok();
            show_win32_balloon_notification(&title, &body);
        }
        #[cfg(not(windows))]
        {
            let _ = (title, body);
        }
    });
}

#[cfg(windows)]
fn encode_wide_fixed<const N: usize>(s: &str) -> [u16; N] {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    let mut buf = [0u16; N];
    for (i, ch) in OsStr::new(s).encode_wide().take(N.saturating_sub(1)).enumerate() {
        buf[i] = ch;
    }
    buf
}

#[cfg(windows)]
fn to_wide_null(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn show_win32_balloon_notification(title: &str, body: &str) {
    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct NotifyIconDataW {
        cb_size: u32,
        h_wnd: isize,
        u_id: u32,
        u_flags: u32,
        u_callback_message: u32,
        h_icon: isize,
        sz_tip: [u16; 128],
        dw_state: u32,
        dw_state_mask: u32,
        sz_info: [u16; 256],
        u_timeout_or_version: u32,
        sz_info_title: [u16; 64],
        dw_info_flags: u32,
        guid_item: Guid,
        h_balloon_icon: isize,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn LoadIconW(h_instance: isize, icon_name: *const u16) -> isize;
        fn MessageBeep(u_type: u32) -> i32;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn Shell_NotifyIconW(dw_message: u32, lp_data: *const NotifyIconDataW) -> i32;
    }

    const NIM_ADD: u32 = 0x0000_0000;
    const NIM_MODIFY: u32 = 0x0000_0001;
    const NIM_DELETE: u32 = 0x0000_0002;
    const NIF_ICON: u32 = 0x0000_0002;
    const NIF_TIP: u32 = 0x0000_0004;
    const NIF_INFO: u32 = 0x0000_0010;
    const NIIF_INFO: u32 = 0x0000_0001;
    const IDI_INFORMATION: usize = 32516;
    const MB_OK: u32 = 0x0000_0000;

    let win_title = to_wide_null(WINDOW_TITLE);
    unsafe {
        let _ = MessageBeep(MB_OK);
        let hwnd = FindWindowW(std::ptr::null(), win_title.as_ptr());
        let h_icon = LoadIconW(0, IDI_INFORMATION as *const u16);

        let nid = NotifyIconDataW {
            cb_size: std::mem::size_of::<NotifyIconDataW>() as u32,
            h_wnd: hwnd,
            u_id: 0x4944_4D01,
            u_flags: NIF_ICON | NIF_TIP | NIF_INFO,
            u_callback_message: 0,
            h_icon,
            sz_tip: encode_wide_fixed::<128>(WINDOW_TITLE),
            dw_state: 0,
            dw_state_mask: 0,
            sz_info: encode_wide_fixed::<256>(body),
            u_timeout_or_version: 4000,
            sz_info_title: encode_wide_fixed::<64>(title),
            dw_info_flags: NIIF_INFO,
            guid_item: Guid {
                data1: 0,
                data2: 0,
                data3: 0,
                data4: [0; 8],
            },
            h_balloon_icon: 0,
        };

        if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
        std::thread::sleep(std::time::Duration::from_secs(4));
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

/// 构造开机自启注册表写入值（带引号包裹的可执行文件路径）
pub fn format_autostart_reg_value(exe_path: &Path) -> String {
    format!("\"{}\"", exe_path.display())
}

/// 开启或关闭 Windows 开机自启动（直接通过 Win32 `advapi32.dll` 内存读写 `HKCU\...\Run`，不调用外部 `reg.exe`）
pub fn apply_autostart(enabled: bool) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        #[link(name = "advapi32")]
        unsafe extern "system" {
            fn RegOpenKeyExW(
                h_key: isize,
                lp_sub_key: *const u16,
                ul_options: u32,
                sam_desired: u32,
                phk_result: *mut isize,
            ) -> i32;
            fn RegSetValueExW(
                h_key: isize,
                lp_value_name: *const u16,
                reserved: u32,
                dw_type: u32,
                lp_data: *const u8,
                cb_data: u32,
            ) -> i32;
            fn RegDeleteValueW(h_key: isize, lp_value_name: *const u16) -> i32;
            fn RegCloseKey(h_key: isize) -> i32;
        }

        const HKEY_CURRENT_USER: isize = -0x7FFF_FFFFisize; // 0x80000001
        const KEY_SET_VALUE: u32 = 0x0002;
        const REG_SZ: u32 = 1;

        let exe = std::env::current_exe()?;
        let subkey = to_wide_null(AUTOSTART_SUBKEY);
        let val_name = to_wide_null(AUTOSTART_VALUE_NAME);

        unsafe {
            let mut hkey: isize = 0;
            let rc = RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut hkey,
            );
            if rc != 0 {
                return Err(std::io::Error::from_raw_os_error(rc));
            }

            if enabled {
                let data_str = format_autostart_reg_value(&exe);
                let data_wide = to_wide_null(&data_str);
                let byte_len = (data_wide.len() * 2) as u32;
                let set_rc = RegSetValueExW(
                    hkey,
                    val_name.as_ptr(),
                    0,
                    REG_SZ,
                    data_wide.as_ptr() as *const u8,
                    byte_len,
                );
                let _ = RegCloseKey(hkey);
                if set_rc != 0 {
                    return Err(std::io::Error::from_raw_os_error(set_rc));
                }
            } else {
                let _ = RegDeleteValueW(hkey, val_name.as_ptr());
                let _ = RegCloseKey(hkey);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_autostart_reg_value() {
        let exe = PathBuf::from(r"C:\Apps\idm-desktop.exe");
        assert_eq!(
            format_autostart_reg_value(&exe),
            "\"C:\\Apps\\idm-desktop.exe\""
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_encode_wide_fixed_truncation_and_null_termination() {
        let buf = encode_wide_fixed::<8>("IDM-Aria2-RS");
        assert_eq!(buf[7], 0);
        assert_eq!(buf[0], b'I' as u16);
    }
}
