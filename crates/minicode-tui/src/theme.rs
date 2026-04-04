/// Theme and color management for the TUI
use ratatui::style::{Color, Modifier, Style};

/// Color theme for different UI elements
#[derive(Debug, Clone, Copy)]
pub struct ColorTheme {
    /// Header workspace section
    pub header: Color,
    /// Main session area
    pub session: Color,
    /// Input section
    pub input: Color,
    /// Approval dialog
    pub approval: Color,
    /// User messages
    pub user: Color,
    /// Assistant messages
    pub assistant: Color,
    /// Progress messages
    pub progress: Color,
    /// Tool messages
    pub tool: Color,
    /// Tool error messages
    pub tool_error: Color,
    /// Command highlights
    pub command_highlight: Color,
    /// Expandable items (like [展开])
    pub expandable: Color,
    // Header labels
    pub header_label_project: Color,
    pub header_label_provider: Color,
    pub header_label_model: Color,
    pub header_label_auth: Color,
    pub header_label_session: Color,
    pub header_label_permissions: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            header: Color::LightCyan,
            session: Color::Blue,
            input: Color::Green,
            approval: Color::LightRed,
            user: Color::Cyan,
            assistant: Color::Green,
            progress: Color::Yellow,
            tool: Color::Magenta,
            tool_error: Color::Red,
            command_highlight: Color::Rgb(30, 50, 80),
            expandable: Color::LightCyan,
            header_label_project: Color::Cyan,
            header_label_provider: Color::LightBlue,
            header_label_model: Color::Green,
            header_label_auth: Color::LightYellow,
            header_label_session: Color::Yellow,
            header_label_permissions: Color::LightMagenta,
        }
    }
}

impl ColorTheme {
    /// Get style for header
    pub fn header_style(&self) -> Style {
        Style::default().fg(self.header)
    }

    /// Get style for session area
    pub fn session_style(&self) -> Style {
        Style::default().fg(self.session)
    }

    /// Get style for input area
    pub fn input_style(&self) -> Style {
        Style::default().fg(self.input)
    }

    /// Get style for approval dialog
    pub fn approval_style(&self) -> Style {
        Style::default().fg(self.approval)
    }

    /// Get style for user messages
    pub fn user_style(&self) -> Style {
        Style::default().fg(self.user)
    }

    /// Get style for assistant messages
    pub fn assistant_style(&self) -> Style {
        Style::default().fg(self.assistant)
    }

    /// Get style for progress messages
    pub fn progress_style(&self) -> Style {
        Style::default().fg(self.progress)
    }

    /// Get style for tool messages
    pub fn tool_style(&self) -> Style {
        Style::default().fg(self.tool)
    }

    /// Get style for tool error messages
    pub fn tool_error_style(&self) -> Style {
        Style::default().fg(self.tool_error)
    }

    /// Get style for expandable items
    pub fn expandable_style(&self) -> Style {
        Style::default()
            .fg(self.expandable)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for header project label
    pub fn header_label_project_style(&self) -> Style {
        Style::default()
            .fg(self.header_label_project)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for header provider label
    pub fn header_label_provider_style(&self) -> Style {
        Style::default()
            .fg(self.header_label_provider)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for header model label
    pub fn header_label_model_style(&self) -> Style {
        Style::default()
            .fg(self.header_label_model)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for header auth label
    pub fn header_label_auth_style(&self) -> Style {
        Style::default()
            .fg(self.header_label_auth)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for header session label
    pub fn header_label_session_style(&self) -> Style {
        Style::default()
            .fg(self.header_label_session)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for header permissions label
    pub fn header_label_permissions_style(&self) -> Style {
        Style::default()
            .fg(self.header_label_permissions)
            .add_modifier(Modifier::BOLD)
    }
}

/// Global theme instance
pub fn theme() -> ColorTheme {
    ColorTheme::default()
}
