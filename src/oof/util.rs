/// Util function to skip the two leading long flag hyphens.
pub fn trim_double_hyphen(flag_text: &str) -> &str {
    flag_text.get(2..).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::trim_double_hyphen;

    #[test]
    fn _trim_double_hyphen() {
        assert_eq!(trim_double_hyphen("--flag"), "flag");
        assert_eq!(trim_double_hyphen("--verbose"), "verbose");
        assert_eq!(trim_double_hyphen("--help"), "help");
    }
}
