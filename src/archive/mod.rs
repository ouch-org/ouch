//! Archive compression algorithms

#[cfg(feature = "unrar")]
pub mod rar;
#[cfg(not(feature = "unrar"))]
pub mod rar_stub;
pub mod sevenz;
pub mod tar;
pub mod zip;
