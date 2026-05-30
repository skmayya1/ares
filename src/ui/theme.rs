use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub active_tab: Color,
    pub active_tab_bg: Color,
    pub inactive_tab: Color,
    pub tab_border: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            active_tab: Color::Cyan,
            active_tab_bg: Color::Rgb(25, 55, 60),
            inactive_tab: Color::DarkGray,
            tab_border: Color::Rgb(45, 45, 45),
        }
    }
}

impl Theme {
    pub fn tab_label(&self, title: &str) -> String {
        format!("[{title}]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_labels_use_brackets() {
        let theme = Theme::default();
        assert_eq!(theme.tab_label("Main"), "[Main]");
        assert_eq!(theme.tab_label("Build Authentication"), "[Build Authentication]");
    }
}
