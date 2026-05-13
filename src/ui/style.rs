pub(crate) const RESET: &str = "\x1b[0m";
pub(crate) const BOLD: &str = "\x1b[1m";
pub(crate) const DIM: &str = "\x1b[2m";
pub(crate) const RED: &str = "\x1b[31m";
pub(crate) const GREEN: &str = "\x1b[32m";
pub(crate) const YELLOW: &str = "\x1b[33m";
pub(crate) const BLUE: &str = "\x1b[34m";
pub(crate) const MAGENTA: &str = "\x1b[35m";
pub(crate) const CYAN: &str = "\x1b[36m";
pub(crate) const WHITE: &str = "\x1b[37m";

pub(crate) fn style(input: &str, modifier: &str, color: Option<&str>) -> String {
    let color = color.unwrap_or("");
    format!("{}{}{}{}", modifier, color, input, RESET)
}

pub(crate) fn centered_styled(input: &str, width: usize) -> String {
    let visible_len = visible_width(input);
    if visible_len >= width {
        return input.to_string();
    }
    let padding = (width - visible_len) / 2;
    format!("{}{}", " ".repeat(padding), input)
}

pub(crate) fn visible_width(input: &str) -> usize {
    let mut width = 0;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }
        width += 1;
    }

    width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_width_ignores_ansi_escape_sequences() {
        let styled = style("zitree", BOLD, Some(WHITE));

        assert_eq!(visible_width(&styled), 6);
    }

    #[test]
    fn centered_styled_accounts_for_visible_width() {
        let centered = centered_styled(&style("hi", BOLD, Some(CYAN)), 6);

        assert_eq!(visible_width(&centered), 4);
        assert!(centered.starts_with("  "));
    }
}
