use crate::database::Database;
use crate::services::ProxyService;
use crate::ssh_manager::SshManager;
use std::sync::Arc;

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub proxy_service: ProxyService,
    pub ssh: Arc<SshManager>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let proxy_service = ProxyService::new(db.clone());
        let ssh = Arc::new(SshManager::new());

        Self { db, proxy_service, ssh }
    }
}
