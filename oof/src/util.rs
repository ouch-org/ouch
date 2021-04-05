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
