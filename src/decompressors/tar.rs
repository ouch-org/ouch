use std::{fs, path::{Path, PathBuf}};

use crate::file::File;
use crate::error::OuchResult;

use colored::Colorize;
pub struct Decompressor {}

impl Decompressor {
    pub fn decompress(from: &File, into: &Option<File>) -> OuchResult<()>  {
        let destination_path = match into {
            Some(output) => {
                // Must be None according to the way command-line arg. parsing in Ouch works
                assert_eq!(output.extension, None);
                
                Path::new(&output.path)
            }
            None => Path::new(".")
        };

        if !destination_path.exists() {
            println!("{}: attempting to create folder {:?}.", "info".yellow(), &destination_path);
            std::fs::create_dir_all(destination_path)?;
            println!("{}: directory {:#?} created.", "info".yellow(), fs::canonicalize(&destination_path)?);
        }



        

        Ok(())
    }
}