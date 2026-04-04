use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use minicode_tool::ToolContext;

/// 基于工具上下文解析目标路径，并执行权限校验。
pub async fn resolve_tool_path(
    context: &ToolContext,
    target_path: &str,
    intent: &str,
) -> Result<PathBuf> {
    let base = PathBuf::from(&context.cwd);
    let resolved = base
        .join(target_path)
        .canonicalize()
        .unwrap_or_else(|_| base.join(target_path));

    if context.permissions.is_none() {
        ensure_inside_workspace(&base, &resolved)?;
        return Ok(resolved);
    }

    if let Some(permissions) = &context.permissions {
        permissions
            .ensure_path_access(resolved.to_string_lossy().as_ref(), intent)
            .await?;
    }

    Ok(resolved)
}

/// 确保路径没有逃逸出当前工作区目录。
fn ensure_inside_workspace(root: impl AsRef<Path>, resolved: impl AsRef<Path>) -> Result<()> {
    let Ok(relative) = resolved.as_ref().strip_prefix(root.as_ref()) else {
        return Err(anyhow!(
            "Path escapes workspace: {}",
            resolved.as_ref().display()
        ));
    };
    if relative
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(anyhow!(
            "Path escapes workspace: {}",
            resolved.as_ref().display()
        ));
    }
    Ok(())
}
