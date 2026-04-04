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
            // Morandi color palette (higher saturation version)
            header: Color::Rgb(120, 150, 140),     // Muted teal
            session: Color::Rgb(140, 120, 160),    // Muted purple
            input: Color::Rgb(130, 160, 100),      // Muted sage green
            approval: Color::Rgb(170, 110, 110),   // Muted mauve
            user: Color::Rgb(160, 130, 100),       // Muted warm brown
            assistant: Color::Rgb(100, 150, 150),  // Muted teal-cyan
            progress: Color::Rgb(170, 150, 90),    // Muted mustard
            tool: Color::Rgb(140, 100, 160),       // Muted purple-plum
            tool_error: Color::Rgb(180, 100, 100), // Muted rose
            command_highlight: Color::Rgb(100, 110, 140), // Muted slate-blue
            expandable: Color::Rgb(110, 150, 150), // Muted cyan-gray
            header_label_project: Color::Rgb(110, 150, 140), // Muted teal
            header_label_provider: Color::Rgb(150, 110, 170), // Muted lilac
            header_label_model: Color::Rgb(140, 160, 100), // Muted green
            header_label_auth: Color::Rgb(170, 150, 100), // Muted ochre
            header_label_session: Color::Rgb(160, 120, 100), // Muted terracotta
            header_label_permissions: Color::Rgb(130, 100, 160), // Muted plum
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

    /// Get style for header recent label
    pub fn header_label_recent_style(&self) -> Style {
        Style::default()
            .fg(self.header_label_permissions)
            .add_modifier(Modifier::BOLD)
    }
}

/// Global theme instance
pub fn theme() -> ColorTheme {
    ColorTheme::default()
}
