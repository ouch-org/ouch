// generic landlock implementation https://landlock.io/rust-landlock/landlock/struct.Ruleset.html

use landlock::{
    Access, AccessFs, PathBeneath, PathFd, PathFdError, RestrictionStatus, Ruleset,
    RulesetAttr, RulesetCreatedAttr, RulesetError, ABI,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyRestrictError {
    #[error(transparent)]
    Ruleset(#[from] RulesetError),
    #[error(transparent)]
    AddRule(#[from] PathFdError),
}

pub fn restrict_paths(hierarchies: &[&str]) -> Result<RestrictionStatus, MyRestrictError> {
    // The Landlock ABI should be incremented (and tested) regularly.
    // ABI set to 2 in compatibility with linux 5.19 and higher
    let abi = ABI::V2;
    let access_all = AccessFs::from_all(abi);
    let access_read = AccessFs::from_read(abi);

    Ok(Ruleset::default()
        .handle_access(access_all)?
        .create()?
        // Read-only access to / (entire filesystem).
        .add_rules(landlock::path_beneath_rules(&["/"], access_read))?
        .add_rules(
            hierarchies
                .iter()
                .map::<Result<_, MyRestrictError>, _>(|p| {
                    Ok(PathBeneath::new(PathFd::new(p)?, access_all))
                }),
        )?
        .restrict_self()?)
}


