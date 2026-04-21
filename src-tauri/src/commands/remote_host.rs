use crate::remote_host::{CreateRemoteHostRequest, RemoteHost, UpdateRemoteHostRequest};
use crate::ssh_manager::ActiveRemoteInfo;
use crate::store::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_remote_hosts(state: State<'_, AppState>) -> Result<Vec<RemoteHost>, String> {
    state.db.list_remote_hosts().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_remote_host(
    state: State<'_, AppState>,
    req: CreateRemoteHostRequest,
) -> Result<RemoteHost, String> {
    state.db.create_remote_host(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_remote_host(
    state: State<'_, AppState>,
    req: UpdateRemoteHostRequest,
) -> Result<RemoteHost, String> {
    state.db.update_remote_host(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_remote_host(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // 如果正在连接此主机，先断开
    if let Some(info) = state.ssh.get_active_info() {
        if info.host_id == id {
            state.ssh.disconnect();
        }
    }
    state.db.delete_remote_host(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_remote_host(
    state: State<'_, AppState>,
    id: String,
) -> Result<ActiveRemoteInfo, String> {
    let host = state
        .db
        .get_remote_host(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("远程主机 {id} 不存在"))?;

    state.ssh.connect(&host).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect_remote_host(state: State<'_, AppState>) -> Result<(), String> {
    state.ssh.disconnect();
    Ok(())
}

#[tauri::command]
pub async fn get_active_remote_host(
    state: State<'_, AppState>,
) -> Result<Option<ActiveRemoteInfo>, String> {
    Ok(state.ssh.get_active_info())
}

#[tauri::command]
pub async fn test_remote_connection(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<String, String> {
    use crate::remote_host::RemoteHost;
    let dummy = RemoteHost {
        id: "test".to_string(),
        name: "test".to_string(),
        host,
        port,
        username,
        password,
        created_at: 0,
    };
    let session = crate::ssh_manager::SshSession::connect(&dummy).map_err(|e| e.to_string())?;
    let output = session
        .exec_command("uname -a")
        .unwrap_or_else(|_| "connected".to_string());
    Ok(output.trim().to_string())
}
