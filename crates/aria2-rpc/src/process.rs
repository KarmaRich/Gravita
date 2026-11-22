//! aria2c 子进程托管模块：负责生成启动参数、拉起与安全停止 aria2c 守护进程

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows 隐藏子进程控制台黑框标志
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// aria2c 进程启动配置
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aria2ProcessConfig {
    /// `aria2c` 可执行文件路径（默认从 PATH 查找 `"aria2c"`）
    pub binary_path: PathBuf,
    /// RPC 监听端口
    pub rpc_port: u16,
    /// RPC 鉴权密钥
    pub rpc_secret: Option<String>,
    /// 默认下载保存目录
    pub download_dir: Option<PathBuf>,
    /// 最大同时下载任务数
    pub max_concurrent_downloads: u32,
    /// 单任务默认分片数
    pub split: u32,
    /// 额外透传的命令行参数
    pub extra_args: Vec<String>,
}

impl Default for Aria2ProcessConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("aria2c"),
            rpc_port: 6800,
            rpc_secret: None,
            download_dir: None,
            max_concurrent_downloads: 5,
            split: 16,
            extra_args: Vec::new(),
        }
    }
}

impl Aria2ProcessConfig {
    /// 生成传给 `aria2c` 的完整命令行参数列表
    pub fn build_args(&self) -> Vec<String> {
        let mut args = vec![
            "--enable-rpc=true".to_string(),
            "--rpc-listen-all=false".to_string(),
            format!("--rpc-listen-port={}", self.rpc_port),
            format!("--max-concurrent-downloads={}", self.max_concurrent_downloads),
            format!("--split={}", self.split),
            format!("--max-connection-per-server={}", self.split.min(16)),
            "--continue=true".to_string(),
        ];

        if let Some(secret) = self.rpc_secret.as_deref().filter(|s| !s.is_empty()) {
            args.push(format!("--rpc-secret={secret}"));
        }
        if let Some(dir) = &self.download_dir {
            args.push(format!("--dir={}", dir.display()));
        }
        args.extend(self.extra_args.iter().cloned());
        args
    }

    /// 获取对应的本地 HTTP JSON-RPC 地址
    pub fn rpc_endpoint(&self) -> String {
        format!("http://127.0.0.1:{}/jsonrpc", self.rpc_port)
    }

    /// 自动探测项目内置的 `aria2c` 可执行文件路径（优先 exe 同级目录，其次工作目录，最后回退系统 PATH）
    pub fn resolve_bundled_binary() -> PathBuf {
        let bin_name = if cfg!(windows) { "aria2c.exe" } else { "aria2c" };
        let mut candidates = Vec::new();

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join("resources").join("bin").join(bin_name));
                candidates.push(dir.join(bin_name));
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("resources").join("bin").join(bin_name));
        }

        candidates
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| PathBuf::from(bin_name))
    }
}

/// 托管中的 `aria2c` 子进程句柄，Drop 时自动终止子进程防止孤儿进程残留
pub struct Aria2Process {
    child: Child,
    config: Aria2ProcessConfig,
}

impl Aria2Process {
    /// 按配置静默启动 `aria2c` 守护进程（Windows 下自动隐藏黑框）
    pub fn spawn(config: Aria2ProcessConfig) -> std::io::Result<Self> {
        let args = config.build_args();
        let mut cmd = Command::new(&config.binary_path);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let child = cmd.spawn()?;
        Ok(Self { child, config })
    }

    /// 检查子进程是否仍在运行
    pub fn is_running(&mut self) -> std::io::Result<bool> {
        Ok(self.child.try_wait()?.is_none())
    }

    /// 返回当前进程对应的配置
    pub fn config(&self) -> &Aria2ProcessConfig {
        &self.config
    }

    /// 主动终止 `aria2c` 子进程
    pub fn stop(&mut self) -> std::io::Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
            let _ = self.child.wait();
        }
        Ok(())
    }
}

impl Drop for Aria2Process {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_with_secret_and_dir() {
        let cfg = Aria2ProcessConfig {
            rpc_port: 16801,
            rpc_secret: Some("s3cr3t".to_string()),
            download_dir: Some(PathBuf::from("D:/Downloads")),
            max_concurrent_downloads: 3,
            split: 8,
            extra_args: vec!["--file-allocation=none".to_string()],
            ..Default::default()
        };

        let args = cfg.build_args();
        assert!(args.contains(&"--enable-rpc=true".to_string()));
        assert!(args.contains(&"--rpc-listen-port=16801".to_string()));
        assert!(args.contains(&"--rpc-secret=s3cr3t".to_string()));
        assert!(args.contains(&"--dir=D:/Downloads".to_string()));
        assert!(args.contains(&"--max-concurrent-downloads=3".to_string()));
        assert!(args.contains(&"--split=8".to_string()));
        assert!(args.contains(&"--file-allocation=none".to_string()));
        assert_eq!(cfg.rpc_endpoint(), "http://127.0.0.1:16801/jsonrpc");
    }
}
