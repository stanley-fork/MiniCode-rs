use std::path::{Path, PathBuf};

use crate::runtime_store;

/// 返回 mini-code 配置目录 `~/.mini-code`。
pub fn mini_code_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".mini-code")
}

/// 返回全局设置文件路径。
pub fn mini_code_settings_path() -> PathBuf {
    mini_code_dir().join("settings.json")
}

/// 返回权限存储文件路径。
pub fn mini_code_permissions_path() -> PathBuf {
    let cwd = &runtime_store().cwd;
    let session_id = &runtime_store().session_id;
    project_session_permissions_path(cwd, session_id)
}

/// 返回全局 MCP 配置文件路径。
pub fn mini_code_mcp_path() -> PathBuf {
    mini_code_dir().join("mcp.json")
}

/// 返回项目级 MCP 配置路径。
pub fn project_mcp_path(cwd: impl AsRef<Path>) -> PathBuf {
    cwd.as_ref().join(".mcp.json")
}

/// 返回项目级会话目录: .mini-code/sessions/
pub fn project_sessions_dir(cwd: impl AsRef<Path>) -> PathBuf {
    cwd.as_ref().join(".mini-code/sessions")
}

/// 返回特定会话目录: .mini-code/sessions/{session_id}/
pub fn project_session_dir(cwd: impl AsRef<Path>, session_id: &str) -> PathBuf {
    project_sessions_dir(cwd).join(session_id)
}

/// 返回会话索引路径: .mini-code/sessions/index.json
pub fn project_sessions_index(cwd: impl AsRef<Path>) -> PathBuf {
    project_sessions_dir(cwd).join("index.json")
}

/// 返回会话元数据路径: .mini-code/sessions/{session_id}/metadata.json
pub fn project_session_metadata_path(cwd: impl AsRef<Path>, session_id: &str) -> PathBuf {
    project_session_dir(cwd, session_id).join("metadata.json")
}

/// 返回会话对话历史路径: .mini-code/sessions/{session_id}/conversation.toml
pub fn project_session_conversation_path(cwd: impl AsRef<Path>, session_id: &str) -> PathBuf {
    project_session_dir(cwd, session_id).join("conversation.toml")
}

/// 返回会话权限文件路径: .mini-code/sessions/{session_id}/permissions.json
pub fn project_session_permissions_path(cwd: impl AsRef<Path>, session_id: &str) -> PathBuf {
    project_session_dir(cwd, session_id).join("permissions.json")
}

/// 返回当前会话文件路径: .mini-code/current_session.json
pub fn project_current_session_path(cwd: impl AsRef<Path>) -> PathBuf {
    cwd.as_ref().join(".mini-code/current_session.json")
}
