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

mod approval;
mod header;
mod transcript;
mod ui_utils;

use approval::build_approval_lines;
use header::build_header_lines;
use transcript::{build_activity_items, transcript_lines};
use ui_utils::{centered_rect, input_viewport, sanitize_line};

pub(crate) fn render_screen(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    args: &TuiAppArgs,
    state: &ScreenState,
) -> Result<()> {
    let visible_commands = get_visible_commands(&state.input);
    let command_rows = if visible_commands.is_empty() {
        0u16
    } else {
        (visible_commands.len().min(6) + 2) as u16
    };

    terminal.draw(|frame| {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(10),
                Constraint::Length(command_rows),
                Constraint::Length(4),
            ])
            .split(area);

        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
            .split(chunks[1]);

        let header = Paragraph::new(build_header_lines(args, state))
            .block(
                Block::default()
                    .title(" MiniCode ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(Color::LightCyan)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(header, chunks[0]);

        let feed_lines = transcript_lines(&state.transcript);
        let fallback = vec![Line::from(
            "(no messages yet, enter /help to list commands)",
        )];
        let feed = Paragraph::new(if feed_lines.is_empty() {
            fallback
        } else {
            feed_lines
        })
        .block(
            Block::default()
                .title(" Session Feed ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::Blue)),
        )
        .wrap(Wrap { trim: false })
        .scroll((state.transcript_scroll_offset as u16, 0));
        frame.render_widget(feed, mid[0]);

        let activity = List::new(build_activity_items(state)).block(
            Block::default()
                .title(" Activity ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::Magenta)),
        );
        frame.render_widget(activity, mid[1]);

        if command_rows > 0 {
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
            if !visible_commands.is_empty() {
                list_state.select(Some(
                    state
                        .selected_slash_index
                        .min(visible_commands.len().min(6) - 1),
                ));
            }

            let commands = List::new(items)
                .block(
                    Block::default()
                        .title(" Slash Commands ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(Color::LightBlue)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(30, 50, 80))
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");
            frame.render_stateful_widget(commands, chunks[2], &mut list_state);
        }

        let prompt_input = sanitize_line(&state.input);
        let input_box = chunks[3];
        let available_input_width = input_box.width.saturating_sub(14) as usize;
        let (display_input, cursor_dx) = input_viewport(
            &prompt_input,
            state.cursor_offset,
            available_input_width.max(1),
        );

        let prompt_text = vec![
            Line::from(format!(
                "status: {}{}{}{}{}",
                state.status.clone().unwrap_or_else(|| "Ready".to_string()),
                state
                    .active_tool
                    .as_ref()
                    .map(|x| format!(" | active={}", x))
                    .unwrap_or_default(),
                if state.is_busy {
                    " | busy".to_string()
                } else {
                    String::new()
                },
                if state.transcript_scroll_offset > 0 {
                    format!(" | scroll={}", state.transcript_scroll_offset)
                } else {
                    String::new()
                },
                {
                    let running_shells = minicode_background_tasks::list_background_tasks()
                        .into_iter()
                        .filter(|task| task.status == "running")
                        .count();
                    format!(" | tools=on | skills=on | shells={}", running_shells)
                }
            ))
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(vec![
                Span::styled(
                    "mini-code> ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(display_input),
            ]),
            Line::from(Span::styled(
                "Enter submit | Tab complete | PgUp/PgDn scroll | Ctrl+C exit",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let prompt = Paragraph::new(prompt_text)
            .block(
                Block::default()
                    .title(" Input ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(Color::Green)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(prompt, input_box);

        let prompt_area: Rect = input_box;
        let prefix_width = display_width("mini-code> ") as u16;
        let cursor_x = (prompt_area.x + 1 + prefix_width + cursor_dx as u16)
            .min(prompt_area.x + prompt_area.width.saturating_sub(2));
        let cursor_y =
            (prompt_area.y + 2).min(prompt_area.y + prompt_area.height.saturating_sub(1));

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
                        .style(Style::default().fg(Color::LightRed)),
                )
                .wrap(Wrap { trim: true });
            frame.render_widget(dialog, popup);
        }

        frame.set_cursor_position((cursor_x, cursor_y));
    })?;
    Ok(())
}
