//! Ouch's argparsing crate.
//!
//! The usage of this crate is heavily based on boolean_flags and
//! argument_flags, there should be an more _obvious_ naming.

mod error;
mod flags;
pub mod util;

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
};

pub use error::OofError;
pub use flags::{ArgFlag, Flag, FlagType, Flags};
use util::trim_double_hyphen;

/// Pop leading application `subcommand`, if valid.
///
/// `args` can be a Vec of `OsString` or `OsStr`
/// `subcommands` is any container that can yield `&str` through `AsRef`, can be `Vec<&str>` or
/// a GREAT `BTreeSet<String>` (or `BTreeSet<&str>`).
pub fn pop_subcommand<'a, T, I, II>(args: &mut Vec<T>, subcommands: I) -> Option<&'a II>
where
    I: IntoIterator<Item = &'a II>,
    II: AsRef<str>,
    T: AsRef<OsStr>,
{
    if args.is_empty() {
        return None;
    }

    for subcommand in subcommands.into_iter() {
        if subcommand.as_ref() == args[0].as_ref() {
            args.remove(0);
            return Some(subcommand);
        }
    }

    None
}

/// Detect flags from args and filter from args.
///
/// Each flag received via flags_info should must have unique long and short identifiers.
///
/// # Panics (Developer errors)
/// - If there are duplicated short flag identifiers.
/// - If there are duplicated long flag identifiers.
///
/// Both conditions cause panic because your program's flags specification is meant to have unique
/// flags. There shouldn't be two "--verbose" flags, for example.
/// Caller should guarantee it, fortunately, this can almost always be caught while prototyping in
/// debug mode, test your CLI flags once, if it works once, you're good,
///
/// # Errors (User errors)
/// - Argument flag comes at last arg, so there's no way to provide an argument.
///     - Or if it doesn't comes at last, but the rest are just flags, no possible and valid arg.
/// - Short flags with multiple letters in the same arg contain a argument flag that does not come
/// as the last one in the list (example "-oahc", where 'o', 'a', or 'h' is a argument flag, but do
/// not comes at last, so it is impossible for them to receive the required argument.
/// - User passes same flag twice (short or long, boolean or arg).
///
/// ...
pub fn filter_flags(
    args: Vec<OsString>,
    flags_info: &[Flag],
) -> Result<(Vec<OsString>, Flags), OofError> {
    let mut short_flags_info = BTreeMap::<char, &Flag>::new();
    let mut long_flags_info = BTreeMap::<&'static str, &Flag>::new();

    for flag in flags_info.iter() {
        // Panics if duplicated/conflicts
        assert!(
            !long_flags_info.contains_key(flag.long),
            "DEV ERROR: duplicated long flag '{}'.",
            flag.long
        );

        long_flags_info.insert(flag.long, flag);

        if let Some(short) = flag.short {
            // Panics if duplicated/conflicts
            assert!(
                !short_flags_info.contains_key(&short),
                "DEV ERROR: duplicated short flag '-{}'.",
                short
            );
            short_flags_info.insert(short, flag);
        }
    }

    // Consume args, filter out flags, and add back to args new vec
    let mut iter = args.into_iter();
    let mut new_args = vec![];
    let mut result_flags = Flags::new();

    while let Some(arg) = iter.next() {
        let flag_type = FlagType::from(&arg);

        // If it isn't a flag, retrieve to `args` and skip this iteration
        if let FlagType::None = flag_type {
            new_args.push(arg);
            continue;
        }

        // If it is a flag, now we try to interpret it as valid utf-8
        let flag = match arg.to_str() {
            Some(arg) => arg,
            None => return Err(OofError::InvalidUnicode(arg)),
        };

        // Only one hyphen in the flag
        // A short flag can be of form "-", "-abcd", "-h", "-v", etc
        if let FlagType::Short = flag_type {
            assert_eq!(flag.chars().next(), Some('-'));

            // TODO
            // TODO: what should happen if the flag is empty?????
            // if flags.chars().skip(1).next().is_none() {
            //     panic!("User error: flag is empty???");
            // }

            // Skip hyphen and get all letters
            let letters = flag.chars().skip(1).collect::<Vec<char>>();

            // For each letter in the short arg, except the last one
            for (i, letter) in letters.iter().copied().enumerate() {
                // Safety: this loop only runs when len >= 1, so this subtraction is safe
                let is_last_letter = i == letters.len() - 1;

                let flag_info =
                    *short_flags_info.get(&letter).ok_or(OofError::UnknownShortFlag(letter))?;

                if !is_last_letter && flag_info.takes_value {
                    return Err(OofError::MisplacedShortArgFlagError(letter));
                    // Because "-AB argument" only works if B takes values, not A.
                    // That is, the short flag that takes values needs to come at the end
                    // of this piece of text
                }

                let flag_name: &'static str = flag_info.long;

                if flag_info.takes_value {
                    // If it was already inserted
                    if result_flags.argument_flags.contains_key(flag_name) {
                        return Err(OofError::DuplicatedFlag(flag_info.clone()));
                    }

                    // pop the next one
                    let flag_argument = iter
                        .next()
                        .ok_or_else(|| OofError::MissingValueToFlag(flag_info.clone()))?;

                    // Otherwise, insert it.
                    result_flags.argument_flags.insert(flag_name, flag_argument);
                } else {
                    // If it was already inserted
                    if result_flags.boolean_flags.contains(flag_name) {
                        return Err(OofError::DuplicatedFlag(flag_info.clone()));
                    }
                    // Otherwise, insert it
                    result_flags.boolean_flags.insert(flag_name);
                }
            }
        }

        if let FlagType::Long = flag_type {
            let flag = trim_double_hyphen(flag);

            let flag_info = *long_flags_info
                .get(flag)
                .ok_or_else(|| OofError::UnknownLongFlag(String::from(flag)))?;

            let flag_name = flag_info.long;

            if flag_info.takes_value {
                // If it was already inserted
                if result_flags.argument_flags.contains_key(&flag_name) {
                    return Err(OofError::DuplicatedFlag(flag_info.clone()));
                }

                let flag_argument =
                    iter.next().ok_or_else(|| OofError::MissingValueToFlag(flag_info.clone()))?;
                result_flags.argument_flags.insert(flag_name, flag_argument);
            } else {
                // If it was already inserted
                if result_flags.boolean_flags.contains(&flag_name) {
                    return Err(OofError::DuplicatedFlag(flag_info.clone()));
                }
                // Otherwise, insert it
                result_flags.boolean_flags.insert(flag_name);
            }

            // // TODO
            // TODO: what should happen if the flag is empty?????
            // if flag.is_empty() {
            //     panic!("Is this an error?");
            // }
        }
    }

    Ok((new_args, result_flags))
}

/// Says if any text matches any arg
pub fn matches_any_arg<T, U>(args: &[T], texts: &[U]) -> bool
where
    T: AsRef<OsStr>,
    U: AsRef<str>,
{
    texts.iter().any(|text| args.iter().any(|arg| arg.as_ref() == text.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_args(text: &str) -> Vec<OsString> {
        let args = text.split_whitespace();
        args.map(OsString::from).collect()
    }

    fn setup_args_scenario(arg_str: &str) -> Result<(Vec<OsString>, Flags), OofError> {
        let flags_info = [
            ArgFlag::long("output_file").short('o'),
            Flag::long("verbose").short('v'),
            Flag::long("help").short('h'),
        ];

        let args = gen_args(arg_str);
        filter_flags(args, &flags_info)
    }

    #[test]
    fn test_unknown_flags() {
        let result = setup_args_scenario("ouch a.zip -s b.tar.gz c.tar").unwrap_err();
        assert!(matches!(result, OofError::UnknownShortFlag(flag) if flag == 's'));

        let unknown_long_flag = "foobar".to_string();
        let result = setup_args_scenario("ouch a.zip --foobar b.tar.gz c.tar").unwrap_err();

        assert!(matches!(result, OofError::UnknownLongFlag(flag) if flag == unknown_long_flag));
    }

    #[test]
    fn test_incomplete_flags() {
        let incomplete_flag = ArgFlag::long("output_file").short('o');
        let result = setup_args_scenario("ouch a.zip b.tar.gz c.tar -o").unwrap_err();

        assert!(matches!(result, OofError::MissingValueToFlag(flag) if flag == incomplete_flag));
    }

    #[test]
    fn test_duplicated_flags() {
        let duplicated_flag = ArgFlag::long("output_file").short('o');
        let result = setup_args_scenario("ouch a.zip b.tar.gz c.tar -o -o -o").unwrap_err();

        assert!(matches!(result, OofError::DuplicatedFlag(flag) if flag == duplicated_flag));
    }

    #[test]
    fn test_misplaced_flag() {
        let misplaced_flag = ArgFlag::long("output_file").short('o');
        let result = setup_args_scenario("ouch -ov a.zip b.tar.gz c.tar").unwrap_err();

        assert!(
            matches!(result, OofError::MisplacedShortArgFlagError(flag) if flag == misplaced_flag.short.unwrap())
        );
    }

    // #[test]
    // fn test_invalid_unicode_flag() {
    //     use std::os::unix::prelude::OsStringExt;

    //     // `invalid_unicode_flag` has to contain a leading hyphen to be considered a flag.
    //     let invalid_unicode_flag = OsString::from_vec(vec![45, 0, 0, 0, 255, 255, 255, 255]);
    //     let result = filter_flags(vec![invalid_unicode_flag.clone()], &[]).unwrap_err();

    //     assert!(matches!(result, OofError::InvalidUnicode(flag) if flag == invalid_unicode_flag));
    // }

    // asdasdsa
    #[test]
    fn test_filter_flags() {
        let flags_info = [
            ArgFlag::long("output_file").short('o'),
            Flag::long("verbose").short('v'),
            Flag::long("help").short('h'),
        ];
        let args = gen_args("ouch a.zip -v b.tar.gz --output_file new_folder c.tar");

        let (args, mut flags) = filter_flags(args, &flags_info).unwrap();

        assert_eq!(args, gen_args("ouch a.zip b.tar.gz c.tar"));
        assert!(flags.is_present("output_file"));
        assert_eq!(Some(&OsString::from("new_folder")), flags.arg("output_file"));
        assert_eq!(Some(OsString::from("new_folder")), flags.take_arg("output_file"));
        assert!(!flags.is_present("output_file"));
    }

    #[test]
    fn test_pop_subcommand() {
        let subcommands = &["commit", "add", "push", "remote"];
        let mut args = gen_args("add a b c");

        let result = pop_subcommand(&mut args, subcommands);

        assert_eq!(result, Some(&"add"));
        assert_eq!(args[0], "a");

        // Check when no subcommand matches
        let mut args = gen_args("a b c");
        let result = pop_subcommand(&mut args, subcommands);

        assert_eq!(result, None);
        assert_eq!(args[0], "a");
    }

    // #[test]
    // fn test_flag_info_macros() {
    //     let flags_info = [
    //         arg_flag!('o', "output_file"),
    //         arg_flag!("delay"),
    //         flag!('v', "verbose"),
    //         flag!('h', "help"),
    //         flag!("version"),
    //     ];

    //     let expected = [
    //         ArgFlag::long("output_file").short('o'),
    //         ArgFlag::long("delay"),
    //         Flag::long("verbose").short('v'),
    //         Flag::long("help").short('h'),
    //         Flag::long("version"),
    //     ];

    //     assert_eq!(flags_info, expected);
    // }

    #[test]
    // TODO: remove should_panic and use proper error handling inside of filter_args
    #[should_panic]
    fn test_flag_info_with_long_flag_conflict() {
        let flags_info = [ArgFlag::long("verbose").short('a'), Flag::long("verbose").short('b')];

        // Should panic here
        let result = filter_flags(vec![], &flags_info);
        assert!(matches!(result, Err(OofError::FlagValueConflict { .. })));
    }

    #[test]
    // TODO: remove should_panic and use proper error handling inside of filter_args
    #[should_panic]
    fn test_flag_info_with_short_flag_conflict() {
        let flags_info =
            [ArgFlag::long("output_file").short('o'), Flag::long("verbose").short('o')];

        // Should panic here
        filter_flags(vec![], &flags_info).unwrap_err();
    }

    #[test]
    fn test_matches_any_arg_function() {
        let args = gen_args("program a -h b");
        assert!(matches_any_arg(&args, &["--help", "-h"]));

        let args = gen_args("program a b --help");
        assert!(matches_any_arg(&args, &["--help", "-h"]));

        let args = gen_args("--version program a b");
        assert!(matches_any_arg(&args, &["--version", "-v"]));

        let args = gen_args("program -v a --version b");
        assert!(matches_any_arg(&args, &["--version", "-v"]));

        // Cases without it
        let args = gen_args("program a b c");
        assert!(!matches_any_arg(&args, &["--help", "-h"]));

        let args = gen_args("program a --version -v b c");
        assert!(!matches_any_arg(&args, &["--help", "-h"]));
    }
}

/// Create a flag with long flag (?).
#[macro_export]
macro_rules! flag {
    ($short:expr, $long:expr) => {{
        oof::Flag::long($long).short($short)
    }};

    ($long:expr) => {{
        oof::Flag::long($long)
    }};
}

/// Create a flag with long flag (?), receives argument (?).
#[macro_export]
macro_rules! arg_flag {
    ($short:expr, $long:expr) => {{
        oof::ArgFlag::long($long).short($short)
    }};

    ($long:expr) => {{
        oof::ArgFlag::long($long)
    }};
}
