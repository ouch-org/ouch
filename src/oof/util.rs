/// Util function to skip the two leading long flag hyphens.
pub fn trim_double_hyphen(flag_text: &str) -> &str {
    let mut chars = flag_text.chars();
    chars.nth(1); // Skipping 2 chars
    chars.as_str()
}

// Currently unused
/// Util function to skip the single leading short flag hyphen.
pub fn trim_single_hyphen(flag_text: &str) -> &str {
    let mut chars = flag_text.chars();

    chars.next(); // Skipping 1 char
    chars.as_str()
}

#[cfg(test)]
mod tests {
    use super::trim_double_hyphen;
    use super::trim_single_hyphen;

    #[test]
    fn _trim_double_hyphen() {
        assert_eq!(trim_double_hyphen("--flag"), "flag");
        assert_eq!(trim_double_hyphen("--verbose"), "verbose");
        assert_eq!(trim_double_hyphen("--help"), "help");
    }

    fn _trim_single_hyphen() {
        assert_eq!(trim_single_hyphen("-vv"), "vv");
        assert_eq!(trim_single_hyphen("-h"), "h");
    }
}
