use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// 密码明文存储（本地 SQLite，仅供个人使用）
    pub password: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateRemoteHostRequest {
    pub name: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRemoteHostRequest {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
}

impl Database {
    pub fn ensure_remote_hosts_table(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "CREATE TABLE IF NOT EXISTS remote_hosts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 22,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn list_remote_hosts(&self) -> Result<Vec<RemoteHost>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, name, host, port, username, password, created_at
                 FROM remote_hosts ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let hosts = stmt
            .query_map([], |row| {
                Ok(RemoteHost {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get::<_, i64>(3)? as u16,
                    username: row.get(4)?,
                    password: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(hosts)
    }

    pub fn get_remote_host(&self, id: &str) -> Result<Option<RemoteHost>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT id, name, host, port, username, password, created_at
             FROM remote_hosts WHERE id = ?1",
            params![id],
            |row| {
                Ok(RemoteHost {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get::<_, i64>(3)? as u16,
                    username: row.get(4)?,
                    password: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        );

        match result {
            Ok(host) => Ok(Some(host)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn create_remote_host(&self, req: CreateRemoteHostRequest) -> Result<RemoteHost, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let port = req.port.unwrap_or(22);
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO remote_hosts (id, name, host, port, username, password, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, req.name, req.host, port as i64, req.username, req.password, created_at],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(RemoteHost {
            id,
            name: req.name,
            host: req.host,
            port,
            username: req.username,
            password: req.password,
            created_at,
        })
    }

    pub fn update_remote_host(&self, req: UpdateRemoteHostRequest) -> Result<RemoteHost, AppError> {
        let port = req.port.unwrap_or(22);
        let conn = lock_conn!(self.conn);
        let rows = conn
            .execute(
                "UPDATE remote_hosts SET name=?2, host=?3, port=?4, username=?5, password=?6
                 WHERE id=?1",
                params![req.id, req.name, req.host, port as i64, req.username, req.password],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        if rows == 0 {
            return Err(AppError::Config(format!("远程主机 {} 不存在", req.id)));
        }

        let created_at: i64 = conn
            .query_row(
                "SELECT created_at FROM remote_hosts WHERE id=?1",
                params![req.id],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(RemoteHost {
            id: req.id,
            name: req.name,
            host: req.host,
            port,
            username: req.username,
            password: req.password,
            created_at,
        })
    }

    pub fn delete_remote_host(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM remote_hosts WHERE id=?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
