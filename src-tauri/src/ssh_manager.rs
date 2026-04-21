use crate::error::AppError;
use crate::remote_host::RemoteHost;
use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRemoteInfo {
    pub host_id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    pub remote_home: String,
}

pub struct SshSession {
    pub info: ActiveRemoteInfo,
    session: Arc<Mutex<Session>>,
}

impl SshSession {
    pub fn connect(host: &RemoteHost) -> Result<Self, AppError> {
        let addr = format!("{}:{}", host.host, host.port);
        let tcp = TcpStream::connect(&addr).map_err(|e| {
            AppError::Config(format!("无法连接到 {addr}: {e}"))
        })?;

        let mut session = Session::new().map_err(|e| {
            AppError::Config(format!("创建 SSH session 失败: {e}"))
        })?;
        session.set_tcp_stream(tcp);
        session.handshake().map_err(|e| {
            AppError::Config(format!("SSH 握手失败: {e}"))
        })?;

        session
            .userauth_password(&host.username, &host.password)
            .map_err(|e| AppError::Config(format!("SSH 认证失败: {e}")))?;

        if !session.authenticated() {
            return Err(AppError::Config("SSH 认证失败：用户名或密码错误".to_string()));
        }

        // 获取远程 home 目录
        let remote_home = Self::exec_command_inner(&session, "echo $HOME")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| format!("/home/{}", host.username));

        let info = ActiveRemoteInfo {
            host_id: host.id.clone(),
            name: host.name.clone(),
            host: host.host.clone(),
            username: host.username.clone(),
            remote_home,
        };

        Ok(Self {
            info,
            session: Arc::new(Mutex::new(session)),
        })
    }

    fn exec_command_inner(session: &Session, cmd: &str) -> Result<String, AppError> {
        let mut channel = session
            .channel_session()
            .map_err(|e| AppError::Config(format!("创建 channel 失败: {e}")))?;
        channel
            .exec(cmd)
            .map_err(|e| AppError::Config(format!("执行命令失败: {e}")))?;
        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| AppError::Config(format!("读取命令输出失败: {e}")))?;
        channel.wait_close().ok();
        Ok(output)
    }

    pub fn exec_command(&self, cmd: &str) -> Result<String, AppError> {
        let session = self
            .session
            .lock()
            .map_err(|e| AppError::Config(format!("SSH session lock 失败: {e}")))?;
        Self::exec_command_inner(&session, cmd)
    }

    /// 将本地路径中的 home 前缀替换为远程 home，得到远程路径
    pub fn to_remote_path(&self, local_path: &Path) -> PathBuf {
        let local_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        if let Ok(rel) = local_path.strip_prefix(&local_home) {
            PathBuf::from(&self.info.remote_home).join(rel)
        } else {
            local_path.to_path_buf()
        }
    }

    pub fn read_file(&self, remote_path: &Path) -> Result<String, AppError> {
        let session = self
            .session
            .lock()
            .map_err(|e| AppError::Config(format!("SSH session lock 失败: {e}")))?;
        let sftp = session
            .sftp()
            .map_err(|e| AppError::Config(format!("SFTP 初始化失败: {e}")))?;

        let mut file = sftp.open(remote_path).map_err(|e| {
            AppError::Config(format!(
                "远程文件不存在: {}: {e}",
                remote_path.display()
            ))
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| AppError::Config(format!("读取远程文件失败: {e}")))?;
        Ok(content)
    }

    pub fn write_file(&self, remote_path: &Path, content: &[u8]) -> Result<(), AppError> {
        let session = self
            .session
            .lock()
            .map_err(|e| AppError::Config(format!("SSH session lock 失败: {e}")))?;
        let sftp = session
            .sftp()
            .map_err(|e| AppError::Config(format!("SFTP 初始化失败: {e}")))?;

        // 确保父目录存在
        if let Some(parent) = remote_path.parent() {
            let _ = sftp.mkdir(parent, 0o755);
        }

        let mut file = sftp
            .create(remote_path)
            .map_err(|e| AppError::Config(format!("创建远程文件失败: {}: {e}", remote_path.display())))?;
        file.write_all(content)
            .map_err(|e| AppError::Config(format!("写入远程文件失败: {e}")))?;
        Ok(())
    }

    pub fn file_exists(&self, remote_path: &Path) -> bool {
        let Ok(session) = self.session.lock() else {
            return false;
        };
        let Ok(sftp) = session.sftp() else {
            return false;
        };
        sftp.stat(remote_path).is_ok()
    }

    pub fn read_dir(&self, remote_path: &Path) -> Result<Vec<String>, AppError> {
        let session = self
            .session
            .lock()
            .map_err(|e| AppError::Config(format!("SSH session lock 失败: {e}")))?;
        let sftp = session
            .sftp()
            .map_err(|e| AppError::Config(format!("SFTP 初始化失败: {e}")))?;

        let entries = sftp
            .readdir(remote_path)
            .map_err(|e| AppError::Config(format!("读取远程目录失败: {}: {e}", remote_path.display())))?;

        Ok(entries
            .into_iter()
            .filter_map(|(path, _)| {
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .collect())
    }
}

/// 全局活跃 SSH session
pub struct SshManager {
    pub active: Mutex<Option<SshSession>>,
}

impl SshManager {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }

    pub fn connect(&self, host: &RemoteHost) -> Result<ActiveRemoteInfo, AppError> {
        let session = SshSession::connect(host)?;
        let info = session.info.clone();
        let mut active = self
            .active
            .lock()
            .map_err(|e| AppError::Config(format!("SshManager lock 失败: {e}")))?;
        *active = Some(session);
        Ok(info)
    }

    pub fn disconnect(&self) {
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
    }

    pub fn get_active_info(&self) -> Option<ActiveRemoteInfo> {
        self.active
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.info.clone()))
    }

    pub fn is_connected(&self) -> bool {
        self.active
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    /// 读取远程文件（相对于远程 home 的路径由调用方传入）
    pub fn read_file(&self, remote_path: &Path) -> Result<String, AppError> {
        let active = self
            .active
            .lock()
            .map_err(|e| AppError::Config(format!("SshManager lock 失败: {e}")))?;
        match active.as_ref() {
            Some(s) => s.read_file(remote_path),
            None => Err(AppError::Config("未连接到远程主机".to_string())),
        }
    }

    pub fn write_file(&self, remote_path: &Path, content: &[u8]) -> Result<(), AppError> {
        let active = self
            .active
            .lock()
            .map_err(|e| AppError::Config(format!("SshManager lock 失败: {e}")))?;
        match active.as_ref() {
            Some(s) => s.write_file(remote_path, content),
            None => Err(AppError::Config("未连接到远程主机".to_string())),
        }
    }

    pub fn file_exists(&self, remote_path: &Path) -> bool {
        self.active
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.file_exists(remote_path)))
            .unwrap_or(false)
    }

    pub fn to_remote_path(&self, local_path: &Path) -> Option<PathBuf> {
        self.active
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.to_remote_path(local_path)))
    }

    pub fn read_dir(&self, remote_path: &Path) -> Result<Vec<String>, AppError> {
        let active = self
            .active
            .lock()
            .map_err(|e| AppError::Config(format!("SshManager lock 失败: {e}")))?;
        match active.as_ref() {
            Some(s) => s.read_dir(remote_path),
            None => Err(AppError::Config("未连接到远程主机".to_string())),
        }
    }
}
