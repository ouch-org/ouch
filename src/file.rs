use std::{convert::TryFrom, path::PathBuf, str::FromStr};

use crate::error::Error as OuchError;
use crate::error;
use crate::extensions::CompressionExtension;

// pub type File = (PathBuf, CompressionExtension);

// #[derive(Debug)]
// pub struct FileWithExtension {
//     pub extension: CompressionExtension,
//     pub filename: PathBuf,
// }

#[derive(PartialEq, Eq, Debug)]
pub enum File {
    WithExtension((PathBuf, CompressionExtension)),
    WithoutExtension(PathBuf)
}

// impl TryFrom<String> for FileWithExtension {
//     type Error = OuchError;

//     fn try_from(filename: String) -> error::OuchResult<Self> {
//         // Safe to unwrap (infallible operation)
//         let filename = PathBuf::from_str(&filename).unwrap();

//         let os_str = match filename.extension() {
//             Some(os_str) => os_str,
//             None => return Err(OuchError::MissingExtensionError(filename.to_string_lossy().to_string())),
//         };

//         let extension = match CompressionExtension::try_from(os_str.into()) {
//             Ok(ext) => ext,
//             Err(err) => return Err(err),
//         };

//         Ok(Self {
//             filename,
//             extension,
//         })
//     }
// }
