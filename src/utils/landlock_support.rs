//Check Landlock kernel support (Linux kernel >= 5.19)

#[cfg(target_os = "linux")]
pub fn is_landlock_supported() -> bool {
    use std::process::Command;

    if let Ok(output) = Command::new("uname").arg("-r").output() {
        if let Ok(version_str) = String::from_utf8(output.stdout) {
            // Version string is expected to be in "5.19.0-foo" or similar
            let mut parts = version_str.trim().split('.');
            if let (Some(major), Some(minor)) = (parts.next(), parts.next()) {
                if let (Ok(major), Ok(minor)) = (major.parse::<u32>(), minor.parse::<u32>()) {
                    return (major > 5) || (major == 5 && minor >= 19);
                }
            }
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
pub fn is_landlock_supported() -> bool {
    false
}
