use ratatui::layout::{Constraint, Direction, Layout, Rect};
use unicode_width::UnicodeWidthStr;

use crate::input::display_width;

pub(super) fn sanitize_line(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() || *ch == '\t')
        .collect::<String>()
        .replace('\t', "    ")
}

pub(super) fn input_viewport(
    input: &str,
    cursor_offset: usize,
    max_width: usize,
) -> (String, usize) {
    if max_width == 0 {
        return (String::new(), 0);
    }

    let chars = input.chars().collect::<Vec<_>>();
    let cursor = cursor_offset.min(chars.len());

    let mut start = 0usize;
    let mut used = 0usize;
    let mut i = cursor;
    while i > 0 {
        let ch = chars[i - 1];
        let w = UnicodeWidthStr::width(ch.to_string().as_str());
        if used + w > max_width {
            break;
        }
        used += w;
        i -= 1;
        start = i;
    }

    let mut out = String::new();
    let mut out_width = 0usize;
    let mut end = start;
    while end < chars.len() {
        let w = UnicodeWidthStr::width(chars[end].to_string().as_str());
        if out_width + w > max_width {
            break;
        }
        out.push(chars[end]);
        out_width += w;
        end += 1;
    }

    let cursor_text = chars[start..cursor].iter().collect::<String>();
    let cursor_dx = display_width(&cursor_text);
    (out, cursor_dx)
}

pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
