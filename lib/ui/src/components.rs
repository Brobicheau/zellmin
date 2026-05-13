use crate::{centered_styled, style, visible_width, BLUE, BOLD, CYAN, RESET, WHITE};

#[derive(Clone, Copy)]
pub struct PanelLayout {
    width: usize,
    gutter: usize,
}

impl PanelLayout {
    pub fn new(cols: usize) -> Self {
        let width = cols.saturating_sub(4).clamp(16, 96);
        let gutter = cols.saturating_sub(width + 4) / 2;
        Self { width, gutter }
    }
}

#[derive(Clone, Copy)]
pub struct BoxPanel {
    width: usize,
    gutter: usize,
}

impl BoxPanel {
    pub fn new(layout: PanelLayout) -> Self {
        Self {
            width: layout.width,
            gutter: layout.gutter,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn print_top(&self) {
        println!("{}", self.format_border('┌', '─', '┐'));
    }

    pub fn print_bottom(&self) {
        println!("{}", self.format_border('└', '─', '┘'));
    }

    pub fn print_line(&self, content: &str) {
        println!("{}", self.format_line(content));
    }

    pub fn print_centered_line(&self, content: &str) {
        self.print_line(&centered_styled(content, self.width));
    }

    pub fn print_section_header(&self, title: &str) {
        self.print_line("");
        self.print_line(&self.format_section_header(title));
    }

    pub fn print_key_value(&self, label: &str, value: &str) {
        let line = format!(
            "{} {}",
            style(&format!("{}:", label), BOLD, Some(BLUE)),
            style(value, RESET, Some(WHITE))
        );
        self.print_line(&line);
    }

    pub fn print_help(&self, key: &str, title: &str) {
        self.print_line(&format!("{} {}", keycap(key), title));
    }

    pub fn print_status(&self, icon: &str, color: &str, message: &str) {
        self.print_top();
        self.print_line(&status_line(icon, color, message));
        self.print_bottom();
    }

    fn format_section_header(&self, title: &str) -> String {
        let header = format!("{} ", style(title, BOLD, Some(WHITE)));
        let plain_len = title.chars().count() + 2;
        let fill = self.width.saturating_sub(plain_len + 1);
        format!("{}{}", header, "─".repeat(fill))
    }

    fn format_border(&self, left: char, fill: char, right: char) -> String {
        format!(
            "{}{}{}{}",
            " ".repeat(self.gutter),
            left,
            fill.to_string().repeat(self.width + 2),
            right
        )
    }

    fn format_line(&self, content: &str) -> String {
        let visible_len = visible_width(content);
        let padding = self.width.saturating_sub(visible_len);
        format!(
            "{}│ {}{} │",
            " ".repeat(self.gutter),
            content,
            " ".repeat(padding)
        )
    }
}

fn keycap(label: &str) -> String {
    style(&format!("[{}]", label), BOLD, Some(CYAN))
}

fn status_line(icon: &str, color: &str, message: &str) -> String {
    format!(
        "{} {}",
        style(icon, BOLD, Some(color)),
        style(message, BOLD, Some(WHITE))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{style, CYAN};

    #[test]
    fn line_padding_uses_visible_width() {
        let panel = BoxPanel::new(PanelLayout::new(24));
        let line = panel.format_line(&style("branch", BOLD, Some(CYAN)));

        assert!(line.starts_with("│ "));
        assert!(line.ends_with(" │"));
        assert_eq!(visible_width(&line), panel.width + 4);
    }

    #[test]
    fn section_header_fills_remaining_width() {
        let panel = BoxPanel::new(PanelLayout::new(40));
        let line = panel.format_section_header("Configuration");

        assert!(visible_width(&line) <= panel.width);
        assert!(line.contains('─'));
    }
}
