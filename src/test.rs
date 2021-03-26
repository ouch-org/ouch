// TODO: remove tests of CompressionFormat::try_from since that's no longer used anywhere

#[cfg(test)]
mod cli {
    use std::{convert::TryFrom, fs, path::Path};

    use crate::{
        cli::{clap_app, Command, CommandKind::*},
        extension::{CompressionFormat::*, Extension},
        file::File,
    };

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

    #[test]
    fn decompress_files_into_folder() -> crate::Result<()> {
        make_dummy_file("file.zip")?;
        let matches = clap_app().get_matches_from(vec!["ouch", "-i", "file.zip", "-o", "folder/"]);
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command {
                kind: Decompression(vec![File {
                    path: fs::canonicalize("file.zip")?,
                    contents_in_memory: None,
                    extension: Some(Extension::from(Zip))
                }]),
                output: Some(File {
                    path: "folder".into(),
                    contents_in_memory: None,
                    extension: None
                }),
            }
        );

        fs::remove_file("file.zip")?;

        Ok(())
    }

    #[test]
    fn decompress_files() -> crate::Result<()> {
        make_dummy_file("my-cool-file.zip")?;
        make_dummy_file("file.tar")?;
        let matches =
            clap_app().get_matches_from(vec!["ouch", "-i", "my-cool-file.zip", "file.tar"]);
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command {
                kind: Decompression(vec![
                    File {
                        path: fs::canonicalize("my-cool-file.zip")?,
                        contents_in_memory: None,
                        extension: Some(Extension::from(Zip))
                    },
                    File {
                        path: fs::canonicalize("file.tar")?,
                        contents_in_memory: None,
                        extension: Some(Extension::from(Tar))
                    }
                ],),
                output: None,
            }
        );

        fs::remove_file("my-cool-file.zip")?;
        fs::remove_file("file.tar")?;

        Ok(())
    }

    #[test]
    fn compress_files() -> crate::Result<()> {
        make_dummy_file("file")?;
        make_dummy_file("file2.jpeg")?;
        make_dummy_file("file3.ok")?;

        let matches = clap_app().get_matches_from(vec![
            "ouch",
            "-i",
            "file",
            "file2.jpeg",
            "file3.ok",
            "-o",
            "file.tar",
        ]);
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command {
                kind: Compression(vec![
                    fs::canonicalize("file")?,
                    fs::canonicalize("file2.jpeg")?,
                    fs::canonicalize("file3.ok")?
                ]),
                output: Some(File {
                    path: "file.tar".into(),
                    contents_in_memory: None,
                    extension: Some(Extension::from(Tar))
                }),
            }
        );

        fs::remove_file("file")?;
        fs::remove_file("file2.jpeg")?;
        fs::remove_file("file3.ok")?;

        Ok(())
    }
}

#[cfg(test)]
mod cli_errors {

    use std::convert::TryFrom;

    use crate::{
        cli::{clap_app, Command},
        error::crate::{Error, Result},
    };

    #[test]
    fn compress_files() -> crate::Result<()> {
        let matches =
            clap_app().get_matches_from(vec!["ouch", "-i", "a_file", "file2.jpeg", "file3.ok"]);
        let res = Command::try_from(matches);

        assert_eq!(
            res,
            Err(crate::Error::InputsMustHaveBeenDecompressible(
                "a_file".into()
            ))
        );

        Ok(())
    }
}

#[cfg(test)]
mod extension_extraction {
    use std::{convert::TryFrom, path::PathBuf, str::FromStr};

    use crate::{
        error::crate::Result,
        extension::{CompressionFormat, Extension},
    };

    #[test]
    fn zip() -> crate::Result<()> {
        let path = PathBuf::from_str("filename.tar.zip").unwrap();
        assert_eq!(CompressionFormat::try_from(&path)?, CompressionFormat::Zip);

        Ok(())
    }

    #[test]
    fn tar_gz() -> crate::Result<()> {
        let extension = Extension::new("folder.tar.gz")?;

        assert_eq!(
            extension,
            Extension {
                first_ext: Some(CompressionFormat::Tar),
                second_ext: CompressionFormat::Gzip
            }
        );

        Ok(())
    }

    #[test]
    fn tar() -> crate::Result<()> {
        let path = PathBuf::from_str("pictures.tar").unwrap();
        assert_eq!(CompressionFormat::try_from(&path)?, CompressionFormat::Tar);

        Ok(())
    }

    #[test]
    fn gz() -> crate::Result<()> {
        let path = PathBuf::from_str("passwords.tar.gz").unwrap();
        assert_eq!(CompressionFormat::try_from(&path)?, CompressionFormat::Gzip);

        Ok(())
    }

    #[test]
    fn lzma() -> crate::Result<()> {
        let path = PathBuf::from_str("mygame.tar.lzma").unwrap();
        assert_eq!(CompressionFormat::try_from(&path)?, CompressionFormat::Lzma);

        Ok(())
    }

    #[test]
    fn bz() -> crate::Result<()> {
        let path = PathBuf::from_str("songs.tar.bz").unwrap();
        assert_eq!(CompressionFormat::try_from(&path)?, CompressionFormat::Bzip);

        Ok(())
    }
}
