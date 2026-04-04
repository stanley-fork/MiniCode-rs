pub use minicode_agent_core::*;
pub use minicode_install::*;
pub use minicode_manage::*;
pub use minicode_mock_model::*;
pub use minicode_permissions::*;
pub use minicode_tools_runtime::*;
pub use minicode_tui::*;

use std::io::IsTerminal;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use minicode_core::config::load_runtime_config;
use minicode_core::prompt::McpServerSummary;
use minicode_core::types::ModelAdapter;

#[derive(Debug, Parser)]
#[command(
    name = "minicode",
    version,
    about = "MiniCode 命令行",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Install {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },
    Mcp {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Skills {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Help,
}

/// 检查标准输入输出是否都连接到交互式终端。
fn is_interactive_terminal() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// 把 CLI 子命令转换为管理命令参数列表。
fn command_to_management_argv(command: &Commands) -> Option<Vec<String>> {
    match command {
        Commands::Mcp { args } => {
            let mut argv = vec!["mcp".to_string()];
            argv.extend(args.iter().cloned());
            Some(argv)
        }
        Commands::Skills { args } => {
            let mut argv = vec!["skills".to_string()];
            argv.extend(args.iter().cloned());
            Some(argv)
        }
        Commands::Help => Some(vec!["help".to_string()]),
        Commands::Install { .. } => None,
    }
}

/// 将日志文本按字符数截断，超出时追加省略号。
fn truncate_log_text(input: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in input.chars().take(max_chars) {
        out.push(ch);
    }
    if input.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

/// 输出 MCP 服务启动阶段的摘要日志。
fn log_mcp_bootstrap(servers: &[McpServerSummary]) {
    eprintln!(
        "\x1b[1;34m[bootstrap]\x1b[0m \x1b[1mMCP servers configured:\x1b[0m {}",
        servers.len()
    );
    if servers.is_empty() {
        return;
    }

    for server in servers {
        let protocol = server
            .protocol
            .as_ref()
            .map(|x| format!(", protocol={x}"))
            .unwrap_or_default();
        let error = server
            .error
            .as_ref()
            .map(|x| format!(", error={}", truncate_log_text(x, 220)))
            .unwrap_or_default();
        let status_colored = match server.status.as_str() {
            "connected" => "\x1b[32mconnected\x1b[0m".to_string(),
            "error" => "\x1b[31merror\x1b[0m".to_string(),
            "disabled" => "\x1b[90mdisabled\x1b[0m".to_string(),
            _ => server.status.clone(),
        };
        eprintln!(
            "\x1b[1;34m[bootstrap]\x1b[0m MCP {}: status={}, tools={}, resources={}, prompts={}{}{}",
            server.name,
            status_colored,
            server.tool_count,
            server.resource_count.unwrap_or(0),
            server.prompt_count.unwrap_or(0),
            protocol,
            error
        );
    }
}

#[tokio::main]
/// 程序入口，失败时以非零状态退出。
async fn main() {
    if let Err(err) = real_main().await {
        let _ = err;
        std::process::exit(1);
    }
}

/// 执行主流程：解析参数、初始化运行时并启动 TUI。
async fn real_main() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cli = Cli::parse();

    if matches!(cli.command, Some(Commands::Install { .. })) {
        run_install_wizard(&cwd)?;
        return Ok(());
    }

    if let Some(command) = &cli.command
        && let Some(management_argv) = command_to_management_argv(command)
        && maybe_handle_management_command(&cwd, &management_argv).await?
    {
        return Ok(());
    }

    let runtime = load_runtime_config(&cwd).ok();
    let tools = Arc::new(create_default_tool_registry(&cwd, runtime.as_ref()).await?);
    let permissions = PermissionManager::new(cwd.clone())?;

    let model: Arc<dyn ModelAdapter> =
        if std::env::var("MINI_CODE_MODEL_MODE").ok().as_deref() == Some("mock") {
            Arc::new(MockModelAdapter)
        } else {
            Arc::new(AnthropicModelAdapter::new(tools.clone(), cwd.clone()))
        };

    let stdin_tty = std::io::stdin().is_terminal();
    let stdout_tty = std::io::stdout().is_terminal();
    let interactive = is_interactive_terminal();

    if !interactive {
        return Err(anyhow!(
            "当前仅支持 ratatui 交互模式：需要在 TTY 终端中运行（stdin_tty={}, stdout_tty={}）。",
            stdin_tty,
            stdout_tty
        ));
    }

    let mcp_servers = tools.get_mcp_servers();
    log_mcp_bootstrap(&mcp_servers);
    set_mcp_startup_logging_enabled(false);

    run_tui_app(TuiAppArgs {
        runtime: runtime.clone(),
        tools: tools.clone(),
        model,
        cwd: cwd.clone(),
        permissions,
    })
    .await?;

    tools.dispose().await;
    Ok(())
}
