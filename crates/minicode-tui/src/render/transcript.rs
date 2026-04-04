use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::state::{ScreenState, TranscriptEntry};

use super::ui_utils::sanitize_line;

const TOOL_PREVIEW_LINES: usize = 6;
const TOOL_PREVIEW_CHARS: usize = 180;

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if idx >= max_chars.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

fn is_tool_entry(entry: &TranscriptEntry) -> bool {
    entry.kind == "tool" || entry.kind == "tool:error"
}

fn transcript_title_line(kind: &str) -> Line<'static> {
    let (label, color) = match kind {
        "assistant" => ("assistant", Color::Green),
        "user" => ("you", Color::Cyan),
        "progress" => ("progress", Color::Yellow),
        "tool:error" => ("tool err", Color::Red),
        "tool" => ("tool", Color::Magenta),
        _ => (kind, Color::Gray),
    };
    Line::from(vec![
        Span::styled("▌", Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(
            label.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

pub(super) struct SessionRender {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) toggle_targets: Vec<(usize, usize)>,
}

pub(super) fn transcript_lines(state: &ScreenState) -> SessionRender {
    let mut lines = Vec::new();
    let mut toggle_targets = Vec::new();
    for (idx, entry) in state.transcript.iter().enumerate() {
        if idx > 0 {
            lines.push(Line::from(""));
        }

        if is_tool_entry(entry) {
            let expanded = state.expanded_tool_entries.contains(&idx);
            let body_lines = entry.body.lines().collect::<Vec<_>>();
            let total = body_lines.len();
            let mut truncated = false;
            let collapsed_preview_len = total.min(TOOL_PREVIEW_LINES);
            let collapsible_by_lines = total > collapsed_preview_len;
            let mut collapsible_by_chars = false;
            if !collapsible_by_lines {
                for line in body_lines.iter().take(collapsed_preview_len) {
                    if sanitize_line(line).chars().count() > TOOL_PREVIEW_CHARS {
                        collapsible_by_chars = true;
                        break;
                    }
                }
            }
            let can_toggle = collapsible_by_lines || collapsible_by_chars;
            let preview_len = if expanded {
                total
            } else {
                total.min(TOOL_PREVIEW_LINES)
            };
            let hidden = total.saturating_sub(preview_len);
            let (label, color) = if entry.kind == "tool:error" {
                ("tool err", Color::Red)
            } else {
                ("tool", Color::Magenta)
            };
            let title_line_idx = lines.len();
            let mut title_spans = vec![
                Span::styled("▌", Style::default().fg(color)),
                Span::raw(" "),
                Span::styled(
                    label.to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ];
            if can_toggle {
                title_spans.push(Span::raw("  "));
                title_spans.push(Span::styled(
                    if expanded { "[收起]" } else { "[展开]" },
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ));
                if !expanded {
                    title_spans.push(Span::raw(format!("  ({} more lines)", hidden)));
                }
                toggle_targets.push((title_line_idx, idx));
            }
            lines.push(Line::from(title_spans));

            for line in body_lines.iter().take(preview_len) {
                let sanitized = sanitize_line(line);
                let display = if expanded {
                    sanitized
                } else {
                    let clipped = truncate_chars(&sanitized, TOOL_PREVIEW_CHARS);
                    if clipped != sanitized {
                        truncated = true;
                    }
                    clipped
                };
                lines.push(Line::from(format!("  {}", display)));
            }
            if !expanded && hidden == 0 && truncated {
                lines.push(Line::from("  ..."));
            }
        } else {
            lines.push(transcript_title_line(&entry.kind));
            for line in entry.body.lines() {
                lines.push(Line::from(format!("  {}", sanitize_line(line))));
            }
        }
    }
    SessionRender {
        lines,
        toggle_targets,
    }
}

pub(super) fn session_lines(state: &ScreenState) -> SessionRender {
    transcript_lines(state)
}
