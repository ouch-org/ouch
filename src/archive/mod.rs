//! Archive compression algorithms

#[cfg(feature = "unrar")]
pub mod rar;
pub mod sevenz;
pub mod tar;
pub mod zip;
