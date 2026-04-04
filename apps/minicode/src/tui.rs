use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::cursor::Show;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::agent_loop::{AgentTurnCallbacks, run_agent_turn};
use crate::cli_commands::{SLASH_COMMANDS, find_matching_slash_commands, try_handle_local_command};
use crate::config::RuntimeConfig;
use crate::history::{load_history_entries, save_history_entries};
use crate::permissions::PermissionManager;
use crate::prompt::build_system_prompt;
use crate::tool::{ToolContext, ToolRegistry};
use crate::types::{ChatMessage, ModelAdapter};

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Show)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
    }
}

#[derive(Clone)]
struct TranscriptEntry {
    kind: String,
    body: String,
}

#[derive(Default)]
struct ScreenState {
    input: String,
    cursor_offset: usize,
    transcript: Vec<TranscriptEntry>,
    transcript_scroll_offset: usize,
    selected_slash_index: usize,
    status: Option<String>,
    active_tool: Option<String>,
    recent_tools: Vec<(String, bool)>,
    history: Vec<String>,
    history_index: usize,
    history_draft: String,
    is_busy: bool,
    message_count: usize,
}

pub struct TuiAppArgs {
    pub runtime: Option<RuntimeConfig>,
    pub tools: Arc<ToolRegistry>,
    pub model: Arc<dyn ModelAdapter>,
    pub cwd: PathBuf,
    pub permissions: PermissionManager,
}

struct TuiCallbacks<'a> {
    state: &'a mut ScreenState,
}

impl<'a> AgentTurnCallbacks for TuiCallbacks<'a> {
    fn on_tool_start(&mut self, tool_name: &str, input: &serde_json::Value) {
        self.state.active_tool = Some(tool_name.to_string());
        self.state.status = Some(format!("Running {tool_name}..."));
        self.state.transcript.push(TranscriptEntry {
            kind: "tool".to_string(),
            body: format!("{}\n{}", tool_name, summarize_tool_input(tool_name, input)),
        });
        self.state.transcript_scroll_offset = 0;
    }

    fn on_tool_result(&mut self, tool_name: &str, output: &str, is_error: bool) {
        self.state
            .recent_tools
            .push((tool_name.to_string(), !is_error));
        self.state.transcript.push(TranscriptEntry {
            kind: if is_error {
                "tool:error".to_string()
            } else {
                "tool".to_string()
            },
            body: output.to_string(),
        });
        self.state.transcript_scroll_offset = 0;
    }

    fn on_assistant_message(&mut self, content: &str) {
        self.state.transcript.push(TranscriptEntry {
            kind: "assistant".to_string(),
            body: content.to_string(),
        });
        self.state.transcript_scroll_offset = 0;
    }

    fn on_progress_message(&mut self, content: &str) {
        self.state.transcript.push(TranscriptEntry {
            kind: "progress".to_string(),
            body: content.to_string(),
        });
        self.state.transcript_scroll_offset = 0;
    }
}

fn summarize_tool_input(tool_name: &str, input: &serde_json::Value) -> String {
    if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
        return format!("{} path={}", tool_name, path);
    }
    if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
        return format!("{} {}", tool_name, command);
    }
    serde_json::to_string(input).unwrap_or_else(|_| "(invalid input)".to_string())
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn byte_index_from_char_offset(value: &str, char_offset: usize) -> usize {
    if char_offset == 0 {
        return 0;
    }
    match value.char_indices().nth(char_offset) {
        Some((index, _)) => index,
        None => value.len(),
    }
}

fn insert_char_at(value: &mut String, char_offset: usize, ch: char) {
    let index = byte_index_from_char_offset(value, char_offset);
    value.insert(index, ch);
}

fn remove_char_before(value: &mut String, char_offset: usize) -> bool {
    if char_offset == 0 {
        return false;
    }
    let start = byte_index_from_char_offset(value, char_offset - 1);
    let end = byte_index_from_char_offset(value, char_offset);
    value.replace_range(start..end, "");
    true
}

fn remove_char_at(value: &mut String, char_offset: usize) -> bool {
    if char_offset >= char_len(value) {
        return false;
    }
    let start = byte_index_from_char_offset(value, char_offset);
    let end = byte_index_from_char_offset(value, char_offset + 1);
    value.replace_range(start..end, "");
    true
}

fn get_visible_commands(input: &str) -> Vec<&'static crate::cli_commands::SlashCommand> {
    if !input.starts_with('/') {
        return vec![];
    }
    if input == "/" {
        return SLASH_COMMANDS.iter().collect();
    }
    let matches = find_matching_slash_commands(input);
    SLASH_COMMANDS
        .iter()
        .filter(|cmd| matches.contains(&cmd.usage.to_string()))
        .collect()
}

fn history_up(state: &mut ScreenState) -> bool {
    if state.history.is_empty() || state.history_index == 0 {
        return false;
    }
    if state.history_index == state.history.len() {
        state.history_draft = state.input.clone();
    }
    state.history_index -= 1;
    state.input = state.history[state.history_index].clone();
    state.cursor_offset = char_len(&state.input);
    true
}

fn history_down(state: &mut ScreenState) -> bool {
    if state.history_index >= state.history.len() {
        return false;
    }
    state.history_index += 1;
    if state.history_index == state.history.len() {
        state.input = state.history_draft.clone();
    } else {
        state.input = state.history[state.history_index].clone();
    }
    state.cursor_offset = char_len(&state.input);
    true
}

fn get_transcript_window_size() -> usize {
    let (_, rows) = crossterm::terminal::size().unwrap_or((120, 40));
    rows.saturating_sub(14).max(8) as usize
}

fn get_transcript_max_scroll_offset(entries: &[TranscriptEntry]) -> usize {
    if entries.is_empty() {
        return 0;
    }
    let line_count = entries
        .iter()
        .map(|e| 2 + e.body.lines().count())
        .sum::<usize>();
    line_count.saturating_sub(get_transcript_window_size())
}

fn scroll_transcript_by(state: &mut ScreenState, delta: isize) -> bool {
    let max = get_transcript_max_scroll_offset(&state.transcript) as isize;
    let next = (state.transcript_scroll_offset as isize + delta).clamp(0, max) as usize;
    if next == state.transcript_scroll_offset {
        return false;
    }
    state.transcript_scroll_offset = next;
    true
}
fn sanitize_line(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() || *ch == '\t')
        .collect::<String>()
        .replace('\t', "    ")
}

fn build_header_lines(args: &TuiAppArgs, state: &ScreenState) -> Vec<Line<'static>> {
    let model = args
        .runtime
        .as_ref()
        .map(|x| x.model.clone())
        .unwrap_or_else(|| "(unconfigured)".to_string());
    let recent = state
        .recent_tools
        .iter()
        .rev()
        .take(3)
        .map(|(name, ok)| format!("{}:{}", name, if *ok { "ok" } else { "err" }))
        .collect::<Vec<_>>()
        .join(", ");

    vec![
        Line::from(format!("model={} | cwd={}", model, args.cwd.display())),
        Line::from(args.permissions.get_summary().join(" | ")),
        Line::from(format!(
            "transcript={} messages={} skills={} mcp={}{}",
            state.transcript.len(),
            state.message_count,
            args.tools.get_skills().len(),
            args.tools.get_mcp_servers().len(),
            if recent.is_empty() {
                String::new()
            } else {
                format!(" | recent={}", recent)
            }
        )),
    ]
}

fn render_screen(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    args: &TuiAppArgs,
    state: &ScreenState,
) -> Result<()> {
    let visible_commands = get_visible_commands(&state.input);
    let command_rows = if visible_commands.is_empty() {
        0u16
    } else {
        (visible_commands.len().min(5) + 2) as u16
    };

    terminal.draw(|frame| {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(8),
                Constraint::Length(command_rows),
                Constraint::Length(3),
            ])
            .split(area);

        let header = Paragraph::new(build_header_lines(args, state))
            .block(Block::default().title("MiniCode-RS").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(header, chunks[0]);

        let feed_lines = render_transcript_lines(&state.transcript)
            .into_iter()
            .map(|line| Line::from(sanitize_line(&line)))
            .collect::<Vec<_>>();
        let fallback = vec![Line::from("(暂无消息，输入 /help 查看命令)")];
        let feed = Paragraph::new(if feed_lines.is_empty() {
            fallback
        } else {
            feed_lines
        })
        .block(Block::default().title("Session Feed").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((state.transcript_scroll_offset as u16, 0));
        frame.render_widget(feed, chunks[1]);

        if command_rows > 0 {
            let items = visible_commands
                .iter()
                .take(5)
                .enumerate()
                .map(|(idx, cmd)| {
                    let marker = if idx == state.selected_slash_index {
                        ">"
                    } else {
                        " "
                    };
                    ListItem::new(format!("{} {} - {}", marker, cmd.usage, cmd.description))
                })
                .collect::<Vec<_>>();
            let commands =
                List::new(items).block(Block::default().title("Commands").borders(Borders::ALL));
            frame.render_widget(commands, chunks[2]);
        }

        let prompt_input = sanitize_line(&state.input);
        let prompt_text = vec![
            Line::from(format!(
                "status: {}{}",
                state.status.clone().unwrap_or_else(|| "Ready".to_string()),
                state
                    .active_tool
                    .as_ref()
                    .map(|x| format!(" | active={}", x))
                    .unwrap_or_default()
            ))
            .style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(format!("mini-code> {}", prompt_input)),
        ];
        let prompt = Paragraph::new(prompt_text)
            .block(Block::default().title("Input").borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(prompt, chunks[3]);

        let prompt_area: Rect = chunks[3];
        let prefix_width = display_width("mini-code> ") as u16;
        let cursor_text = state
            .input
            .chars()
            .take(state.cursor_offset.min(char_len(&state.input)))
            .collect::<String>();
        let cursor_dx = display_width(&cursor_text) as u16;
        let cursor_x = (prompt_area.x + 1 + prefix_width + cursor_dx)
            .min(prompt_area.x + prompt_area.width.saturating_sub(2));
        let cursor_y =
            (prompt_area.y + 2).min(prompt_area.y + prompt_area.height.saturating_sub(1));
        frame.set_cursor_position((cursor_x, cursor_y));
    })?;
    Ok(())
}

fn render_transcript_lines(entries: &[TranscriptEntry]) -> Vec<String> {
    let mut lines = vec![];
    for (idx, entry) in entries.iter().enumerate() {
        if idx > 0 {
            lines.push(String::new());
        }
        lines.push(format!("[{}]", entry.kind));
        for line in entry.body.lines() {
            lines.push(format!("  {}", line));
        }
    }
    lines
}

fn parse_shortcut_command(input: &str) -> (Option<&'static str>, serde_json::Value) {
    if input == "/ls" {
        return (Some("list_files"), serde_json::json!({ "path": "." }));
    }
    if let Some(path) = input.strip_prefix("/ls ") {
        return (
            Some("list_files"),
            serde_json::json!({ "path": path.trim() }),
        );
    }

    if let Some(rest) = input.strip_prefix("/grep ") {
        let parts = rest.split("::").collect::<Vec<_>>();
        if parts.len() == 2 {
            return (
                Some("grep_files"),
                serde_json::json!({ "pattern": parts[0].trim(), "path": parts[1].trim() }),
            );
        }
        return (
            Some("grep_files"),
            serde_json::json!({ "pattern": rest.trim() }),
        );
    }

    if let Some(path) = input.strip_prefix("/read ") {
        return (
            Some("read_file"),
            serde_json::json!({ "path": path.trim() }),
        );
    }

    if let Some(rest) = input.strip_prefix("/write ") {
        let parts = rest.splitn(2, "::").collect::<Vec<_>>();
        if parts.len() == 2 {
            return (
                Some("write_file"),
                serde_json::json!({ "path": parts[0].trim(), "content": parts[1] }),
            );
        }
    }

    if let Some(rest) = input.strip_prefix("/modify ") {
        let parts = rest.splitn(2, "::").collect::<Vec<_>>();
        if parts.len() == 2 {
            return (
                Some("modify_file"),
                serde_json::json!({ "path": parts[0].trim(), "content": parts[1] }),
            );
        }
    }

    if let Some(rest) = input.strip_prefix("/edit ") {
        let parts = rest.splitn(3, "::").collect::<Vec<_>>();
        if parts.len() == 3 {
            return (
                Some("edit_file"),
                serde_json::json!({
                    "path": parts[0].trim(),
                    "search": parts[1],
                    "replace": parts[2]
                }),
            );
        }
    }

    if let Some(rest) = input.strip_prefix("/patch ") {
        let parts = rest.split("::").collect::<Vec<_>>();
        if parts.len() >= 3 && parts.len() % 2 == 1 {
            let path = parts[0].trim();
            let mut replacements = vec![];
            let mut i = 1;
            while i + 1 < parts.len() {
                replacements
                    .push(serde_json::json!({ "search": parts[i], "replace": parts[i + 1] }));
                i += 2;
            }
            return (
                Some("patch_file"),
                serde_json::json!({ "path": path, "replacements": replacements }),
            );
        }
    }

    if let Some(rest) = input.strip_prefix("/cmd ") {
        if let Some((cwd, cmd)) = rest.split_once("::") {
            return (
                Some("run_command"),
                serde_json::json!({ "cwd": cwd.trim(), "command": cmd.trim() }),
            );
        }
        return (
            Some("run_command"),
            serde_json::json!({ "command": rest.trim() }),
        );
    }

    (None, serde_json::Value::Null)
}

async fn handle_submit(
    args: &mut TuiAppArgs,
    state: &mut ScreenState,
    messages: &mut Vec<ChatMessage>,
    raw_input: String,
) -> Result<bool> {
    let input = raw_input.trim().to_string();
    if input.is_empty() {
        return Ok(false);
    }
    if input == "/exit" {
        return Ok(true);
    }

    if state.history.last().map(|x| x.as_str()) != Some(input.as_str()) {
        state.history.push(input.clone());
        let _ = save_history_entries(&state.history);
    }
    state.history_index = state.history.len();
    state.history_draft.clear();

    if input == "/tools" {
        state.transcript.push(TranscriptEntry {
            kind: "assistant".to_string(),
            body: args
                .tools
                .list()
                .iter()
                .map(|tool| format!("{}: {}", tool.name(), tool.description()))
                .collect::<Vec<_>>()
                .join("\n"),
        });
        return Ok(false);
    }

    if let Some(local) = try_handle_local_command(&input, &args.cwd, Some(&args.tools)).await? {
        state.transcript.push(TranscriptEntry {
            kind: "assistant".to_string(),
            body: local,
        });
        return Ok(false);
    }

    let shortcut = parse_shortcut_command(&input);
    if let Some(tool_name) = shortcut.0 {
        state.is_busy = true;
        state.status = Some(format!("Running {tool_name}..."));
        disable_raw_mode()?;
        let result = args
            .tools
            .execute(
                tool_name,
                shortcut.1,
                &ToolContext {
                    cwd: args.cwd.to_string_lossy().to_string(),
                    permissions: Some(Arc::new(args.permissions.clone())),
                },
            )
            .await;
        enable_raw_mode()?;
        state.is_busy = false;
        state.status = None;
        state.transcript.push(TranscriptEntry {
            kind: if result.ok {
                "tool".to_string()
            } else {
                "tool:error".to_string()
            },
            body: result.output,
        });
        return Ok(false);
    }

    if input.starts_with('/') {
        let matches = find_matching_slash_commands(&input);
        state.transcript.push(TranscriptEntry {
            kind: "assistant".to_string(),
            body: if matches.is_empty() {
                "未识别命令。输入 /help 查看可用命令。".to_string()
            } else {
                format!("未识别命令。你是不是想输入：\n{}", matches.join("\n"))
            },
        });
        return Ok(false);
    }

    messages[0] = ChatMessage::System {
        content: build_system_prompt(
            &args.cwd,
            &args.permissions.get_summary(),
            args.tools.get_skills(),
            args.tools.get_mcp_servers(),
        ),
    };
    messages.push(ChatMessage::User {
        content: input.clone(),
    });
    state.transcript.push(TranscriptEntry {
        kind: "user".to_string(),
        body: input,
    });

    args.permissions.begin_turn();
    state.status = Some("Thinking...".to_string());
    state.is_busy = true;

    disable_raw_mode()?;
    let mut callbacks = TuiCallbacks { state };
    let updated = run_agent_turn(
        args.model.as_ref(),
        &args.tools,
        messages.clone(),
        ToolContext {
            cwd: args.cwd.to_string_lossy().to_string(),
            permissions: Some(Arc::new(args.permissions.clone())),
        },
        None,
        Some(&mut callbacks),
    )
    .await;
    enable_raw_mode()?;

    *messages = updated;
    args.permissions.end_turn();
    state.is_busy = false;
    state.status = None;
    state.active_tool = None;
    Ok(false)
}

pub async fn run_tui_app(mut args: TuiAppArgs) -> Result<()> {
    let _terminal_guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut state = ScreenState {
        history: load_history_entries(),
        message_count: 1,
        ..ScreenState::default()
    };
    state.history_index = state.history.len();

    let mut messages = vec![ChatMessage::System {
        content: build_system_prompt(
            &args.cwd,
            &args.permissions.get_summary(),
            args.tools.get_skills(),
            args.tools.get_mcp_servers(),
        ),
    }];

    let mut should_exit = false;
    while !should_exit {
        render_screen(&mut terminal, &args, &state)?;

        if event::poll(Duration::from_millis(150))? {
            match event::read()? {
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        let _ = scroll_transcript_by(&mut state, 3);
                    }
                    MouseEventKind::ScrollDown => {
                        let _ = scroll_transcript_by(&mut state, -3);
                    }
                    _ => {}
                },
                Event::Key(key) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        should_exit = true;
                        continue;
                    }

                    let visible_commands = get_visible_commands(&state.input);

                    match key {
                        KeyEvent {
                            code: KeyCode::Enter,
                            ..
                        } => {
                            if state.is_busy {
                                continue;
                            }
                            if !visible_commands.is_empty() {
                                let selected = visible_commands
                                    .get(state.selected_slash_index.min(visible_commands.len() - 1))
                                    .map(|x| x.usage)
                                    .unwrap_or(state.input.as_str());
                                if state.input.trim() != selected {
                                    state.input = selected.to_string();
                                    state.cursor_offset = char_len(&state.input);
                                    state.selected_slash_index = 0;
                                    continue;
                                }
                            }
                            let submitted = state.input.clone();
                            state.input.clear();
                            state.cursor_offset = 0;
                            state.selected_slash_index = 0;
                            should_exit =
                                handle_submit(&mut args, &mut state, &mut messages, submitted)
                                    .await?;
                            state.message_count = messages.len();
                        }
                        KeyEvent {
                            code: KeyCode::Backspace,
                            ..
                        } => {
                            if remove_char_before(&mut state.input, state.cursor_offset) {
                                state.cursor_offset -= 1;
                            }
                            state.selected_slash_index = 0;
                        }
                        KeyEvent {
                            code: KeyCode::Delete,
                            ..
                        } => {
                            let _ = remove_char_at(&mut state.input, state.cursor_offset);
                            state.selected_slash_index = 0;
                        }
                        KeyEvent {
                            code: KeyCode::Left,
                            ..
                        } => {
                            state.cursor_offset = state.cursor_offset.saturating_sub(1);
                        }
                        KeyEvent {
                            code: KeyCode::Right,
                            ..
                        } => {
                            state.cursor_offset =
                                (state.cursor_offset + 1).min(char_len(&state.input));
                        }
                        KeyEvent {
                            code: KeyCode::PageUp,
                            ..
                        } => {
                            let _ = scroll_transcript_by(&mut state, 8);
                        }
                        KeyEvent {
                            code: KeyCode::PageDown,
                            ..
                        } => {
                            let _ = scroll_transcript_by(&mut state, -8);
                        }
                        KeyEvent {
                            code: KeyCode::Tab, ..
                        } => {
                            if !visible_commands.is_empty() {
                                if let Some(selected) = visible_commands
                                    .get(state.selected_slash_index.min(visible_commands.len() - 1))
                                {
                                    state.input = selected.usage.to_string();
                                    state.cursor_offset = char_len(&state.input);
                                    state.selected_slash_index = 0;
                                }
                            }
                        }
                        KeyEvent {
                            code: KeyCode::Up,
                            modifiers,
                            ..
                        } => {
                            if !visible_commands.is_empty() {
                                state.selected_slash_index =
                                    (state.selected_slash_index + visible_commands.len() - 1)
                                        % visible_commands.len();
                            } else if modifiers.contains(KeyModifiers::ALT) {
                                let _ = scroll_transcript_by(&mut state, 1);
                            } else {
                                let _ = history_up(&mut state);
                            }
                        }
                        KeyEvent {
                            code: KeyCode::Down,
                            modifiers,
                            ..
                        } => {
                            if !visible_commands.is_empty() {
                                state.selected_slash_index =
                                    (state.selected_slash_index + 1) % visible_commands.len();
                            } else if modifiers.contains(KeyModifiers::ALT) {
                                let _ = scroll_transcript_by(&mut state, -1);
                            } else {
                                let _ = history_down(&mut state);
                            }
                        }
                        KeyEvent {
                            code: KeyCode::Home,
                            ..
                        } => {
                            state.cursor_offset = 0;
                        }
                        KeyEvent {
                            code: KeyCode::End, ..
                        } => {
                            state.cursor_offset = char_len(&state.input);
                        }
                        KeyEvent {
                            code: KeyCode::Esc, ..
                        } => {
                            state.input.clear();
                            state.cursor_offset = 0;
                            state.selected_slash_index = 0;
                        }
                        KeyEvent {
                            code: KeyCode::Char('a'),
                            modifiers,
                            ..
                        } if modifiers.contains(KeyModifiers::CONTROL) => {
                            if state.input.is_empty() {
                                state.transcript_scroll_offset =
                                    get_transcript_max_scroll_offset(&state.transcript);
                            } else {
                                state.cursor_offset = 0;
                            }
                        }
                        KeyEvent {
                            code: KeyCode::Char('e'),
                            modifiers,
                            ..
                        } if modifiers.contains(KeyModifiers::CONTROL) => {
                            if state.input.is_empty() {
                                state.transcript_scroll_offset = 0;
                            } else {
                                state.cursor_offset = char_len(&state.input);
                            }
                        }
                        KeyEvent {
                            code: KeyCode::Char('u'),
                            modifiers,
                            ..
                        } if modifiers.contains(KeyModifiers::CONTROL) => {
                            state.input.clear();
                            state.cursor_offset = 0;
                            state.selected_slash_index = 0;
                        }
                        KeyEvent {
                            code: KeyCode::Char('p'),
                            modifiers,
                            ..
                        } if modifiers.contains(KeyModifiers::CONTROL) => {
                            let _ = history_up(&mut state);
                        }
                        KeyEvent {
                            code: KeyCode::Char('n'),
                            modifiers,
                            ..
                        } if modifiers.contains(KeyModifiers::CONTROL) => {
                            let _ = history_down(&mut state);
                        }
                        KeyEvent {
                            code: KeyCode::Char(ch),
                            modifiers,
                            ..
                        } => {
                            if !modifiers.contains(KeyModifiers::CONTROL) {
                                let at = state.cursor_offset.min(char_len(&state.input));
                                insert_char_at(&mut state.input, at, ch);
                                state.cursor_offset = at + 1;
                                state.selected_slash_index = 0;
                                state.history_index = state.history.len();
                            }
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    let _ = save_history_entries(&state.history);
    Ok(())
}
