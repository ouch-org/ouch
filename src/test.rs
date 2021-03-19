#[cfg(test)]
mod cli {
    
    use std::{convert::TryFrom};

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
            vec!["ouch", "-i", "file.zip"]
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
                output: None,
            }
        );

        Ok(())
    }
}