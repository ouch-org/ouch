// Landlock support and generic Landlock sandbox implementation.
// https://landlock.io/rust-landlock/landlock/struct.Ruleset.html

use std::path::Path;

use landlock::{
    Access, AccessFs, PathBeneath, PathFd, PathFdError, RestrictionStatus, Ruleset,
    RulesetAttr, RulesetCreatedAttr, RulesetError, ABI,
};
use thiserror::Error;

/// The status code returned from `ouch` on error
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;

/// Returns true if Landlock is supported by the running kernel (Linux kernel >= 5.19).
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

#[derive(Debug, Error)]
pub enum MyRestrictError {
    #[error(transparent)]
    Ruleset(#[from] RulesetError),
    #[error(transparent)]
    AddRule(#[from] PathFdError),
}

/// Restricts the process to only access the given hierarchies using Landlock, if supported.
///
/// The Landlock ABI is set to v2 for compatibility with Linux 5.19+.
/// All hierarchies are given full access, but root ("/") is read-only.
fn restrict_paths(hierarchies: &[&str]) -> Result<RestrictionStatus, MyRestrictError> {
    // The Landlock ABI should be incremented (and tested) regularly.
    // ABI set to 2 in compatibility with linux 5.19 and higher
    let abi = ABI::V2;
    let access_all = AccessFs::from_all(abi);
    let access_read = AccessFs::from_read(abi);

    let mut ruleset = Ruleset::default()
        .handle_access(access_all)?
        .create()?
        // Read-only access to / (entire filesystem).
        .add_rules(landlock::path_beneath_rules(&["/"], access_read))?;

    // Add write permissions to specified directory of provided
    if !hierarchies.is_empty() {
        ruleset = ruleset.add_rules(
            hierarchies
                .iter()
                .map::<Result<_, MyRestrictError>, _>(|p| {
                    Ok(PathBeneath::new(PathFd::new(p)?, access_all))
                }),
        )?;
    }

    Ok(ruleset.restrict_self()?)
}

/// Restricts the process to only access the given hierarchies using Landlock, if supported.
/// Accepts multiple allowed directories as &[&Path].
pub fn init_sandbox(allowed_dirs: &[&Path], disable_sandbox: bool) {
    // if std::env::var("CI").is_ok() {
    //    return;
    // }
    if disable_sandbox {
        println!("Sandbox feature disabled via --no-sandbox flag.");
        // warn!("Security Process isolation disabled");
        return;
    }

    if is_landlock_supported() {
        let paths: Vec<&str> = allowed_dirs
            .iter()
            .map(|p| p.to_str().expect("Cannot convert path"))
            .collect();

        let status = if !paths.is_empty() {
            restrict_paths(&paths)
        } else {
            restrict_paths(&[])
        };

        match status {
            Ok(_status) => {
                //check
            }
            Err(_e) => {
                //log warning
                std::process::exit(EXIT_FAILURE);
            }
        }
    } else {
        // warn!("Landlock is NOT supported on this platform or kernel (<5.19).");
    }
}
