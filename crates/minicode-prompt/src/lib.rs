use std::path::Path;

use minicode_config::runtime_store;
use minicode_permissions::get_permission_manager;
use minicode_tool::get_tool_registry;

/// 尝试读取文件内容，失败时返回 `None`。
fn maybe_read(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// 组合运行上下文、权限、技能和 MCP 信息，生成系统提示词。
pub fn build_system_prompt() -> String {
    let cwd = runtime_store().cwd.clone();
    let permission_summary = get_permission_manager().get_summary_text();
    let skills = get_tool_registry().get_skills();
    let mcp_servers = get_tool_registry().get_mcp_servers();

    let mut lines = Vec::new();
    lines.push(format!(include_str!("./prompt.txt"), cwd.display()));

    if !permission_summary.is_empty() {
        lines.push(format!(
            "Permission context:\n{}",
            permission_summary.join("\n")
        ));
    }

    if skills.is_empty() {
        lines.push("Available skills:\n- none discovered".to_string());
    } else {
        let skills_text = skills
            .iter()
            .map(|skill| format!("- {}: {}", skill.name, skill.description))
            .collect::<Vec<_>>()
            .join("\n");
        lines.push(format!("Available skills:\n{}", skills_text));
    }

    if !mcp_servers.is_empty() {
        let servers_text = mcp_servers
            .iter()
            .map(|server| {
                let suffix = server
                    .error
                    .as_ref()
                    .map(|x| format!(" ({})", x))
                    .unwrap_or_default();
                let protocol = server
                    .protocol
                    .as_ref()
                    .map(|x| format!(", protocol={}", x))
                    .unwrap_or_default();
                let resources = server
                    .resource_count
                    .map(|x| format!(", resources={}", x))
                    .unwrap_or_default();
                let prompts = server
                    .prompt_count
                    .map(|x| format!(", prompts={}", x))
                    .unwrap_or_default();
                format!(
                    "- {}: {}, tools={}{}{}{}{}",
                    server.name,
                    server.status,
                    server.tool_count,
                    resources,
                    prompts,
                    protocol,
                    suffix
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        lines.push(format!("Configured MCP servers:\n{}", servers_text));

        if mcp_servers.iter().any(|s| s.status == "connected") {
            lines.push(
                "Connected MCP tools are already exposed in the tool list with names prefixed like mcp__server__tool. Use list_mcp_resources/read_mcp_resource and list_mcp_prompts/get_mcp_prompt when a server exposes those capabilities.".to_string(),
            );
        }
    }

    if let Some(home) = dirs::home_dir() {
        let global_path = home.join(".claude").join("CLAUDE.md");
        if let Some(content) = maybe_read(&global_path) {
            lines.push(format!(
                "Global instructions from ~/.claude/CLAUDE.md:\n{}",
                content
            ));
        }
    }

    let project_path = cwd.join("CLAUDE.md");
    if let Some(content) = maybe_read(&project_path) {
        lines.push(format!(
            "Project instructions from {}:\n{}",
            project_path.display(),
            content
        ));
    }

    lines.join("\n\n")
}
