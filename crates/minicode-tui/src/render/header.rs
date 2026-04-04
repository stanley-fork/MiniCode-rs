use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::state::{ScreenState, TuiAppArgs};

pub(super) fn build_header_lines(args: &TuiAppArgs, state: &ScreenState) -> Vec<Line<'static>> {
    let model = args
        .runtime
        .as_ref()
        .map(|x| x.model.clone())
        .unwrap_or_else(|| "(unconfigured)".to_string());
    let provider = args
        .runtime
        .as_ref()
        .map(|x| {
            x.base_url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap_or("custom")
                .to_string()
        })
        .unwrap_or_else(|| "offline".to_string());
    let auth = args
        .runtime
        .as_ref()
        .map(|x| {
            if x.auth_token.is_some() {
                "auth_token"
            } else if x.api_key.is_some() {
                "api_key"
            } else {
                "none"
            }
        })
        .unwrap_or("none");
    let recent = state
        .recent_tools
        .iter()
        .rev()
        .take(3)
        .map(|(name, ok)| format!("{}:{}", name, if *ok { "ok" } else { "err" }))
        .collect::<Vec<_>>()
        .join(", ");

    vec![
        Line::from(vec![
            Span::styled(
                "project",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(args.cwd.display().to_string()),
            Span::raw("   "),
            Span::styled(
                "provider",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(provider),
            Span::raw("   "),
            Span::styled(
                "model",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(model),
            Span::raw("   "),
            Span::styled(
                "auth",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(auth),
        ]),
        Line::from(vec![
            Span::styled(
                "session",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " messages={} events={} skills={} mcp={}",
                state.message_count,
                state.transcript.len(),
                args.tools.get_skills().len(),
                args.tools.get_mcp_servers().len()
            )),
            Span::raw(" | local"),
        ]),
        Line::from(vec![
            Span::styled(
                "permissions",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(args.permissions.get_summary().join(" | ")),
            if recent.is_empty() {
                Span::raw("")
            } else {
                Span::raw(format!(" | recent={}", recent))
            },
        ]),
    ]
}
