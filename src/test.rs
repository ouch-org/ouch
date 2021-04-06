use std::{fs, path::Path};

#[allow(dead_code)]
// ouch's command-line logic uses fs::canonicalize on its inputs so we cannot
// use made-up files for testing.
// make_dummy_file therefores creates a small temporary file to bypass fs::canonicalize errors
fn make_dummy_file<'a, P>(path: P) -> crate::Result<()>
where
    P: AsRef<Path> + 'a,
{
    fs::write(path.as_ref(), &[2, 3, 4, 5, 6, 7, 8, 9, 10])?;
    Ok(())
}

#[allow(dead_code)]
fn make_dummy_files<'a, P>(paths: &[P]) -> crate::Result<()>
where
    P: AsRef<Path> + 'a,
{
    let _ = paths
        .iter()
        .map(make_dummy_file)
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    Ok(())
}

#[cfg(test)]
mod argparsing {
    use super::make_dummy_files;
    use crate::cli;
    use crate::cli::Command;
    use std::{ffi::OsString, fs, path::PathBuf};

    fn gen_args(text: &str) -> Vec<OsString> {
        let args = text.split_whitespace();
        args.map(OsString::from).collect()
    }

    macro_rules! parse {
        ($input_text:expr) => {{
            let args = gen_args($input_text);
            cli::parse_args_from(args).unwrap()
        }};
    }

    #[test]
    // The absolute flags that ignore all the other argparsing rules are --help and --version
    fn test_absolute_flags() {
        let expected = Command::ShowHelp;
        assert_eq!(expected, parse!("").command);
        assert_eq!(expected, parse!("-h").command);
        assert_eq!(expected, parse!("--help").command);
        assert_eq!(expected, parse!("aaaaaaaa --help -o -e aaa").command);
        assert_eq!(expected, parse!("aaaaaaaa -h").command);
        assert_eq!(expected, parse!("--help compress aaaaaaaa").command);
        assert_eq!(expected, parse!("compress --help").command);
        assert_eq!(expected, parse!("--version --help").command);
        assert_eq!(expected, parse!("aaaaaaaa -v aaaa -h").command);

        let expected = Command::ShowVersion;
        assert_eq!(expected, parse!("ouch --version").command);
        assert_eq!(expected, parse!("ouch a --version b").command);
    }

    #[test]
    fn test_arg_parsing_compress_subcommand() -> crate::Result<()> {
        let files = vec!["a", "b", "c"];
        make_dummy_files(&*files)?;
        let files = files
            .iter()
            .map(fs::canonicalize)
            .map(Result::unwrap)
            .collect();

        let expected = Command::Compress {
            files,
            compressed_output_path: "d".into(),
        };
        assert_eq!(expected, parse!("compress a b c d").command);

        fs::remove_file("a")?;
        fs::remove_file("b")?;
        fs::remove_file("c")?;
        Ok(())
    }

    #[test]
    fn test_arg_parsing_decompress_subcommand() -> crate::Result<()> {
        let files = vec!["d", "e", "f"];
        make_dummy_files(&*files)?;
        
        let files: Vec<_> = files.iter().map(PathBuf::from).collect();

        let expected = Command::Decompress {
            files: files
                .iter()
                .map(fs::canonicalize)
                .map(Result::unwrap)
                .collect(),
            output_folder: None,
        };
        
        assert_eq!(expected, parse!("d e f").command);

        let expected = Command::Decompress {
            files: files.iter().map(fs::canonicalize).map(Result::unwrap).collect(),
            output_folder: Some("folder".into()),
        };
        assert_eq!(expected, parse!("d e f --output folder").command);
        assert_eq!(expected, parse!("d e --output folder f").command);
        assert_eq!(expected, parse!("d --output folder e f").command);
        assert_eq!(expected, parse!("--output folder d e f").command);

        assert_eq!(expected, parse!("d e f -o folder").command);
        assert_eq!(expected, parse!("d e -o folder f").command);
        assert_eq!(expected, parse!("d -o folder e f").command);
        assert_eq!(expected, parse!("-o folder d e f").command);

        fs::remove_file("d")?;
        fs::remove_file("e")?;
        fs::remove_file("f")?;
        Ok(())
    }
}

#[cfg(test)]
mod byte_pretty_printing {
    use crate::bytes::Bytes;
    #[test]
    fn bytes() {
        assert_eq!(&format!("{}", Bytes::new(234)), "234.00 B");

        assert_eq!(&format!("{}", Bytes::new(999)), "999.00 B");
    }

    #[test]
    fn kilobytes() {
        assert_eq!(&format!("{}", Bytes::new(2234)), "2.23 kB");

        assert_eq!(&format!("{}", Bytes::new(62500)), "62.50 kB");

        assert_eq!(&format!("{}", Bytes::new(329990)), "329.99 kB");
    }

    #[test]
    fn megabytes() {
        assert_eq!(&format!("{}", Bytes::new(2750000)), "2.75 MB");

        assert_eq!(&format!("{}", Bytes::new(55000000)), "55.00 MB");

        assert_eq!(&format!("{}", Bytes::new(987654321)), "987.65 MB");
    }

    #[test]
    fn gigabytes() {
        assert_eq!(&format!("{}", Bytes::new(5280000000)), "5.28 GB");

        assert_eq!(&format!("{}", Bytes::new(95200000000)), "95.20 GB");

        assert_eq!(&format!("{}", Bytes::new(302000000000)), "302.00 GB");
    }
}

// #[cfg(test)]
// mod cli {
//     use super::*;

//     #[test]
//     fn decompress_files_into_folder() -> crate::Result<()> {
//         make_dummy_file("file.zip")?;
//         let args = gen_args("ouch -i file.zip -o folder/");
//         let (command, flags) = cli::parse_args_and_flags_from(args)?;

//         assert_eq!(
//             command,
//             Command::Decompress {
//                 files: args,
//                 compressed_output_path: PathBuf,
//             } //     kind: Decompress(vec![File {
//               //         path: fs::canonicalize("file.zip")?,
//               //         contents_in_memory: None,
//               //         extension: Some(Extension::from(Zip))
//               //     }]),
//               //     output: Some(File {
//               //         path: "folder".into(),
//               //         contents_in_memory: None,
//               //         extension: None
//               //     }),
//               // }
//         );

//         fs::remove_file("file.zip")?;

//         Ok(())
//     }

//     #[test]
//     fn decompress_files() -> crate::Result<()> {
//         make_dummy_file("my-cool-file.zip")?;
//         make_dummy_file("file.tar")?;
//         let matches =
//             clap_app().get_matches_from(vec!["ouch", "-i", "my-cool-file.zip", "file.tar"]);
//         let command_from_matches = Command::try_from(matches)?;

//         assert_eq!(
//             command_from_matches,
//             Command {
//                 kind: Decompress(vec![
//                     File {
//                         path: fs::canonicalize("my-cool-file.zip")?,
//                         contents_in_memory: None,
//                         extension: Some(Extension::from(Zip))
//                     },
//                     File {
//                         path: fs::canonicalize("file.tar")?,
//                         contents_in_memory: None,
//                         extension: Some(Extension::from(Tar))
//                     }
//                 ],),
//                 output: None,
//             }
//         );

//         fs::remove_file("my-cool-file.zip")?;
//         fs::remove_file("file.tar")?;

//         Ok(())
//     }

//     #[test]
//     fn compress_files() -> crate::Result<()> {
//         make_dummy_file("file")?;
//         make_dummy_file("file2.jpeg")?;
//         make_dummy_file("file3.ok")?;

//         let matches = clap_app().get_matches_from(vec![
//             "ouch",
//             "-i",
//             "file",
//             "file2.jpeg",
//             "file3.ok",
//             "-o",
//             "file.tar",
//         ]);
//         let command_from_matches = Command::try_from(matches)?;

//         assert_eq!(
//             command_from_matches,
//             Command {
//                 kind: Compress(vec![
//                     fs::canonicalize("file")?,
//                     fs::canonicalize("file2.jpeg")?,
//                     fs::canonicalize("file3.ok")?
//                 ]),
//                 output: Some(File {
//                     path: "file.tar".into(),
//                     contents_in_memory: None,
//                     extension: Some(Extension::from(Tar))
//                 }),
//             }
//         );

//         fs::remove_file("file")?;
//         fs::remove_file("file2.jpeg")?;
//         fs::remove_file("file3.ok")?;

//         Ok(())
//     }
// }

// #[cfg(test)]
// mod cli_errors {

//     #[test]
//     fn compress_files() -> crate::Result<()> {
//         let matches =
//             clap_app().get_matches_from(vec!["ouch", "-i", "a_file", "file2.jpeg", "file3.ok"]);
//         let res = Command::try_from(matches);

//         assert_eq!(
//             res,
//             Err(crate::Error::InputsMustHaveBeenDecompressible(
//                 "a_file".into()
//             ))
//         );

//         Ok(())
//     }
// }

// #[cfg(test)]
// mod extension_extraction {

//     #[test]
//     fn test_extension_zip() {
//         let path = "filename.tar.zip";
//         assert_eq!(
//             CompressionFormat::try_from(path),
//             Ok(CompressionFormat::Zip)
//         );
//     }

//     #[test]
//     fn test_extension_tar_gz() {
//         let extension = Extension::from(OsStr::new("folder.tar.gz")).unwrap();
//         assert_eq!(
//             extension,
//             Extension {
//                 first_ext: Some(CompressionFormat::Tar),
//                 second_ext: CompressionFormat::Gzip
//             }
//         );
//     }

//     #[test]
//     fn test_extension_tar() {
//         let path = "pictures.tar";
//         assert_eq!(
//             CompressionFormat::try_from(path),
//             Ok(CompressionFormat::Tar)
//         );
//     }

//     #[test]
//     fn test_extension_gz() {
//         let path = "passwords.tar.gz";
//         assert_eq!(
//             CompressionFormat::try_from(path),
//             Ok(CompressionFormat::Gzip)
//         );
//     }

//     #[test]
//     fn test_extension_lzma() {
//         let path = "mygame.tar.lzma";
//         assert_eq!(
//             CompressionFormat::try_from(path),
//             Ok(CompressionFormat::Lzma)
//         );
//     }

//     #[test]
//     fn test_extension_bz() {
//         let path = "songs.tar.bz";
//         assert_eq!(
//             CompressionFormat::try_from(path),
//             Ok(CompressionFormat::Bzip)
//         );
//     }
// }
