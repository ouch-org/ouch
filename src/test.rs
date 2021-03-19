#[cfg(test)]
mod cli {

    use crate::cli::clap_app;
    use crate::cli::Command;
    use crate::cli::CommandType::*;
    use crate::error::OuchResult;
    use crate::extensions::CompressionExtension::*;
    use crate::file::File;
    use std::convert::TryFrom;

    #[test]
    fn decompress_files_into_folder() -> OuchResult<()> {
        let matches = clap_app().get_matches_from(vec!["ouch", "-i", "file.zip", "-o", "folder/"]);
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command {
                command_type: Decompression(vec![("file.zip".into(), Zip,),],),
                output: Some(File::WithoutExtension("folder".into())),
            }
        );

        Ok(())
    }

    #[test]
    fn decompress_files() -> OuchResult<()> {
        let matches = clap_app().get_matches_from(vec!["ouch", "-i", "file.zip", "file.tar"]);
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command {
                command_type: Decompression(vec![
                    ("file.zip".into(), Zip,),
                    ("file.tar".into(), Tar,),
                ],),
                output: None,
            }
        );

        Ok(())
    }

    #[test]
    fn compress_files() -> OuchResult<()> {
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
                command_type: Compression(vec![
                    "file".into(),
                    "file2.jpeg".into(),
                    "file3.ok".into()
                ]),
                output: Some(File::WithExtension(("file.tar".into(), Tar)))
            }
        );

        Ok(())
    }
}

#[cfg(test)]
mod cli_errors {

    use std::convert::TryFrom;

    use crate::cli::clap_app;
    use crate::cli::Command;
    use crate::error::Error;
    use crate::error::OuchResult;

    #[test]
    fn compress_files() -> OuchResult<()> {
        let matches =
            clap_app().get_matches_from(vec!["ouch", "-i", "a_file", "file2.jpeg", "file3.ok"]);
        let res = Command::try_from(matches);

        assert_eq!(
            res,
            Err(Error::InputsMustHaveBeenDecompressible("a_file".into()))
        );

        Ok(())
    }
}

#[cfg(test)]
mod extension_extraction {
    use crate::error::OuchResult;
    use crate::extensions::CompressionExtension;
    use std::{convert::TryFrom, path::PathBuf, str::FromStr};

    #[test]
    fn zip() -> OuchResult<()> {
        let path = PathBuf::from_str("filename.tar.zip").unwrap();
        assert_eq!(
            CompressionExtension::try_from(&path)?,
            CompressionExtension::Zip
        );

        Ok(())
    }

    #[test]
    fn tar() -> OuchResult<()> {
        let path = PathBuf::from_str("pictures.tar").unwrap();
        assert_eq!(
            CompressionExtension::try_from(&path)?,
            CompressionExtension::Tar
        );

        Ok(())
    }

    #[test]
    fn gz() -> OuchResult<()> {
        let path = PathBuf::from_str("passwords.tar.gz").unwrap();
        assert_eq!(
            CompressionExtension::try_from(&path)?,
            CompressionExtension::Gzip
        );

        Ok(())
    }

    #[test]
    fn lzma() -> OuchResult<()> {
        let path = PathBuf::from_str("mygame.tar.lzma").unwrap();
        assert_eq!(
            CompressionExtension::try_from(&path)?,
            CompressionExtension::Lzma
        );

        Ok(())
    }

    #[test]
    fn bz() -> OuchResult<()> {
        let path = PathBuf::from_str("songs.tar.bz").unwrap();
        assert_eq!(
            CompressionExtension::try_from(&path)?,
            CompressionExtension::Bzip
        );

        Ok(())
    }
}
