//! Archive compression algorithms

#[cfg(not(feature = "bzip3"))]
pub mod bzip3_stub;
#[cfg(feature = "unrar")]
pub mod rar;
#[cfg(not(feature = "unrar"))]
pub mod rar_stub;
pub mod sevenz;
pub mod squashfs;
pub mod tar;
pub mod zip;
