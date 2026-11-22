//! 桌面端系统集成模块：Windows 原生通知与开机自启动管理

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const AUTOSTART_REG_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const AUTOSTART_VALUE_NAME: &str = "IDM-Aria2-RS";

/// 异步发送系统通知（后台线程执行，不阻塞 UI）
pub fn send_system_notification(title: impl Into<String>, body: impl Into<String>) {
    let title = title.into();
    let body = body.into();

    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            let safe_title = escape_ps_single_quoted(&title);
            let safe_body = escape_ps_single_quoted(&body);
            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; \
                 $n = New-Object System.Windows.Forms.NotifyIcon; \
                 $n.Icon = [System.Drawing.SystemIcons]::Information; \
                 $n.Visible = $true; \
                 $n.ShowBalloonTip(4000, '{safe_title}', '{safe_body}', [System.Windows.Forms.ToolTipIcon]::Info); \
                 Start-Sleep -Seconds 5; \
                 $n.Dispose()"
            );
            let mut cmd = Command::new("powershell");
            cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.status();
        }
        #[cfg(not(windows))]
        {
            let _ = (title, body);
        }
    });
}

/// 构造 Windows 注册表开机自启命令参数（便于单元测试验证）
pub fn build_autostart_reg_args(enabled: bool, exe_path: &Path) -> Vec<String> {
    if enabled {
        vec![
            "add".into(),
            AUTOSTART_REG_KEY.into(),
            "/v".into(),
            AUTOSTART_VALUE_NAME.into(),
            "/t".into(),
            "REG_SZ".into(),
            "/d".into(),
            format!("\"{}\"", exe_path.display()),
            "/f".into(),
        ]
    } else {
        vec![
            "delete".into(),
            AUTOSTART_REG_KEY.into(),
            "/v".into(),
            AUTOSTART_VALUE_NAME.into(),
            "/f".into(),
        ]
    }
}

/// 开启或关闭 Windows 开机自启动
pub fn apply_autostart(enabled: bool) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()?;
        let args = build_autostart_reg_args(enabled, &exe);
        let mut cmd = Command::new("reg");
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.status()?;
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
    }
    Ok(())
}

/// PowerShell 单引号字符串转义
fn escape_ps_single_quoted(input: &str) -> String {
    input.replace('\'', "''").replace(['\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_escape_ps_single_quoted() {
        assert_eq!(escape_ps_single_quoted("it's\ndone"), "it''s done");
    }

    #[test]
    fn test_build_autostart_reg_args() {
        let exe = PathBuf::from(r"C:\Apps\idm-desktop.exe");
        let add_args = build_autostart_reg_args(true, &exe);
        assert_eq!(add_args[0], "add");
        assert!(add_args.contains(&"\"C:\\Apps\\idm-desktop.exe\"".to_string()));

        let del_args = build_autostart_reg_args(false, &exe);
        assert_eq!(del_args[0], "delete");
        assert!(del_args.contains(&AUTOSTART_VALUE_NAME.to_string()));
    }
}
