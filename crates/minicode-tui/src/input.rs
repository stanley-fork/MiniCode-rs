use minicode_cli_commands::{SLASH_COMMANDS, SlashCommand, find_matching_slash_commands};
use unicode_width::UnicodeWidthStr;

use crate::state::ScreenState;

pub(crate) fn char_len(value: &str) -> usize {
    value.chars().count()
}

pub(crate) fn display_width(value: &str) -> usize {
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

pub(crate) fn insert_char_at(value: &mut String, char_offset: usize, ch: char) {
    let index = byte_index_from_char_offset(value, char_offset);
    value.insert(index, ch);
}

pub(crate) fn remove_char_before(value: &mut String, char_offset: usize) -> bool {
    if char_offset == 0 {
        return false;
    }
    let start = byte_index_from_char_offset(value, char_offset - 1);
    let end = byte_index_from_char_offset(value, char_offset);
    value.replace_range(start..end, "");
    true
}

pub(crate) fn remove_char_at(value: &mut String, char_offset: usize) -> bool {
    if char_offset >= char_len(value) {
        return false;
    }
    let start = byte_index_from_char_offset(value, char_offset);
    let end = byte_index_from_char_offset(value, char_offset + 1);
    value.replace_range(start..end, "");
    true
}

pub(crate) fn get_visible_commands(input: &str) -> Vec<&'static SlashCommand> {
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

pub(crate) fn history_up(state: &mut ScreenState) -> bool {
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

pub(crate) fn history_down(state: &mut ScreenState) -> bool {
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

pub(crate) fn scroll_transcript_by(state: &mut ScreenState, delta: isize) -> bool {
    let max = state.session_max_scroll_offset as isize;
    let next = (state.transcript_scroll_offset as isize + delta).clamp(0, max) as usize;
    if next == state.transcript_scroll_offset {
        return false;
    }
    state.transcript_scroll_offset = next;
    true
}

pub(crate) fn toggle_tool_details(state: &mut ScreenState, index: usize) -> bool {
    if index >= state.transcript.len() {
        return false;
    }
    if !state.expanded_tool_entries.insert(index) {
        state.expanded_tool_entries.remove(&index);
    }
    true
}
