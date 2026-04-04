use std::io::Stdout;

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListState, Paragraph, Wrap};

use crate::input::{display_width, get_visible_commands};
use crate::state::{ScreenState, TuiAppArgs};
use crate::theme::theme;

mod approval;
mod header;
mod transcript;
mod ui_utils;

use approval::build_approval_lines;
use header::build_header_lines;
use transcript::session_lines;
use ui_utils::{centered_rect, input_viewport, sanitize_line};

pub(crate) fn render_screen(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    args: &TuiAppArgs,
    state: &mut ScreenState,
) -> Result<()> {
    let theme = theme();
    let visible_commands = get_visible_commands(&state.input);
    let command_rows = if visible_commands.is_empty() {
        0u16
    } else {
        (visible_commands.len().min(6) + 2) as u16
    };

    terminal.draw(|frame| {
        let area = frame.area();
        let input_height = if command_rows > 0 { 7 } else { 3 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Min(10),
                Constraint::Length(input_height),
            ])
            .split(area);

        let header = Paragraph::new(build_header_lines(args, state))
            .block(
                Block::default()
                    .title(" Workspace ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(theme.header_style()),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(header, chunks[0]);

        state.visible_tool_toggle_rows.clear();
        let session_render = session_lines(state);
        let feed_line_count = session_render.lines.len();
        let feed_viewport_height = chunks[1].height.saturating_sub(2) as usize;
        let max_scroll = feed_line_count.saturating_sub(feed_viewport_height);
        state.session_max_scroll_offset = max_scroll;
        state.transcript_scroll_offset = state.transcript_scroll_offset.min(max_scroll);
        let scroll_from_bottom = state.transcript_scroll_offset;
        let scroll_from_top = max_scroll.saturating_sub(scroll_from_bottom);
        for (line_idx, entry_idx) in &session_render.toggle_targets {
            if *line_idx >= scroll_from_top && *line_idx < scroll_from_top + feed_viewport_height {
                let row = (*line_idx - scroll_from_top) as u16;
                let screen_y = chunks[1].y + 1 + row;
                if screen_y < chunks[1].y + chunks[1].height.saturating_sub(1) {
                    state.visible_tool_toggle_rows.push((screen_y, *entry_idx));
                }
            }
        }
        let fallback = vec![Line::from(
            "(no messages yet, enter /help to list commands)",
        )];
        let feed_lines = if session_render.lines.is_empty() {
            fallback
        } else {
            session_render.lines
        };
        let scroll_title = if state.transcript_scroll_offset > 0 {
            format!(" Session (scroll: {}) ", state.transcript_scroll_offset)
        } else {
            " Session ".to_string()
        };
        let feed = Paragraph::new(feed_lines)
            .block(
                Block::default()
                    .title(scroll_title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(theme.session_style()),
            )
            .scroll((scroll_from_top as u16, 0));
        frame.render_widget(feed, chunks[1]);

        let status_text = state.status.clone().unwrap_or_else(|| "Ready".to_string());

        let mut status_info = String::from("status: ");
        status_info.push_str(&status_text);

        if let Some(active_tool) = &state.active_tool {
            status_info.push_str(" | active=");
            status_info.push_str(active_tool);
        }

        let prompt_input = sanitize_line(&state.input);
        let input_box = chunks[2];
        let available_input_width = input_box.width.saturating_sub(14) as usize;
        let (display_input, cursor_dx) = input_viewport(
            &prompt_input,
            state.cursor_offset,
            available_input_width.max(1),
        );

        let prompt_text = vec![Line::from(vec![
            Span::styled(
                "mini-code> ",
                Style::default()
                    .fg(theme.input)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(display_input),
        ])];

        // Build block with status on the right
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(theme.input_style());

        // Create title string with left-aligned "Input"
        let input_title = " Input ";
        block = block.title(input_title);

        // Create right-aligned status title
        let status_title = format!(" {} ", status_info);
        block = block.title(status_title);
        let prompt = Paragraph::new(prompt_text)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(prompt, input_box);

        if command_rows > 0 {
            let command_area = Rect {
                x: input_box.x + 1,
                y: input_box.y + input_box.height,
                width: input_box.width.saturating_sub(2),
                height: command_rows.saturating_sub(1),
            };
            let items = visible_commands
                .iter()
                .take(6)
                .map(|cmd| {
                    ratatui::widgets::ListItem::new(Line::from(vec![
                        Span::styled(
                            cmd.usage.to_string(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::raw(cmd.description.to_string()),
                    ]))
                })
                .collect::<Vec<_>>();

            let mut list_state = ListState::default();
            list_state.select(Some(
                state
                    .selected_slash_index
                    .min(visible_commands.len().min(6) - 1),
            ));
            let commands = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(theme.command_highlight)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");
            frame.render_stateful_widget(commands, command_area, &mut list_state);
        }

        let prompt_area: Rect = input_box;
        let prefix_width = display_width("mini-code> ") as u16;
        let cursor_x = (prompt_area.x + 1 + prefix_width + cursor_dx as u16)
            .min(prompt_area.x + prompt_area.width.saturating_sub(2));
        let cursor_y = prompt_area.y + 1;

        if let Some(pending) = &state.pending_approval {
            let popup = centered_rect(70, 45, area);
            frame.render_widget(Clear, popup);
            let lines = build_approval_lines(pending);
            let dialog = Paragraph::new(lines)
                .block(
                    Block::default()
                        .title(" Approval Required ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(theme.approval_style()),
                )
                .wrap(Wrap { trim: true });
            frame.render_widget(dialog, popup);
        }

        frame.set_cursor_position((cursor_x, cursor_y));
    })?;
    Ok(())
}
