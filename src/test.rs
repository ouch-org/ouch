use std::{convert::TryFrom};



#[cfg(test)]
mod cli {

    use std::convert::TryFrom;
    use crate::cli::clap_app;
    use crate::cli::Command;
    use crate::file::File;
    use crate::cli::CommandType::*;
    use crate::extensions::CompressionExtension::*;
    use crate::error::OuchResult;


    #[test]
    fn decompress_files_into_folder() -> OuchResult<()> {
        let matches = clap_app().
        get_matches_from(
            vec!["ouch", "-i", "file.zip", "-o", "folder/"]
        );
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command {
                command_type: Decompression(
                    vec![
                        (
                            "file.zip".into(),
                            Zip,
                        ),
                    ],
                ),
                output: Some(File::WithoutExtension("folder".into())),
            }
        );

        Ok(())
    }

    #[test]
    fn decompress_files() -> OuchResult<()> {
        let matches = clap_app().
        get_matches_from(
            vec!["ouch", "-i", "file.zip", "file.tar"]
        );
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command {
                command_type: Decompression(
                    vec![
                        (
                            "file.zip".into(),
                            Zip,
                        ),
                        (
                            "file.tar".into(),
                            Tar,
                        ),
                    ],
                ),
                output: None,
            }
        );

        Ok(())
    }

    #[test]
    fn compress_files() -> OuchResult<()> {
        let matches = clap_app().
        get_matches_from(
            vec!["ouch", "-i", "file", "file2.jpeg", "file3.ok", "-o", "file.tar"]
        );
        let command_from_matches = Command::try_from(matches)?;

        assert_eq!(
            command_from_matches,
            Command { command_type: Compression(vec!["file".into(), "file2.jpeg".into(), "file3.ok".into()]), output: Some(File::WithExtension(("file.tar".into(), Tar))) }
        );

        Ok(())
    }
}

#[cfg(test)]
mod cli_errors {
    
    use std::convert::TryFrom;

    use crate::cli::clap_app;
    use crate::cli::Command;
    use crate::file::File;
    use crate::cli::CommandType::*;
    use crate::extensions::CompressionExtension::*;
    use crate::error::OuchResult;
    use crate::error::Error;


    #[test]
    fn compress_files() -> OuchResult<()> {
        let matches = clap_app().
        get_matches_from(
            vec!["ouch", "-i", "a_file", "file2.jpeg", "file3.ok"]
        );
        let res = Command::try_from(matches);

        assert_eq!(
            res,
            Err(Error::InputsMustHaveBeenDecompressible("a_file".into()))
        );

        Ok(())
    }
}