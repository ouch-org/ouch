//! Split archives whose volumes are plain byte slices of one archive.

use std::{
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::extension::split_7z_base;

/// Lists all volumes of a split 7z archive from its first volume `archive.7z.001` and nothing for other paths.
pub fn split_7z_volumes(path: &Path) -> io::Result<Vec<PathBuf>> {
    let Some(base) = split_7z_base(path) else {
        return Ok(Vec::new());
    };
    let mut volumes: Vec<PathBuf> = (1..)
        .map(|number: u32| {
            let mut name = base.clone().into_os_string();
            name.push(format!(".{number:03}"));
            PathBuf::from(name)
        })
        .take_while(|volume| volume.is_file())
        .collect();
    if volumes.is_empty() {
        return Ok(volumes);
    }

    // Files after the last volume are left out by the archive length from the start header
    let mut reader = MultiVolumeReader::open(&volumes)?;
    let mut header = [0; 32];
    // A header that cannot be read is reported when the archive itself is read
    if reader.read_exact(&mut header).is_err() {
        return Ok(volumes);
    }
    // The 32 byte start header holds the next header offset at byte 12 and its size at byte 20
    let field = |at: usize| u64::from_le_bytes(header[at..at + 8].try_into().expect("8 bytes"));
    let length = 32u64.saturating_add(field(12)).saturating_add(field(20));
    volumes.truncate(reader.offsets.partition_point(|&start| start < length));
    Ok(volumes)
}

/// Reads files one after another as one seekable stream with only one file open at a time.
pub struct MultiVolumeReader {
    paths: Vec<PathBuf>,
    /// Start offset of each volume followed by the total length
    offsets: Vec<u64>,
    position: u64,
    open_volume: Option<(usize, fs::File)>,
}

impl MultiVolumeReader {
    pub fn open(paths: &[PathBuf]) -> io::Result<Self> {
        let mut offsets = vec![0];
        for path in paths {
            offsets.push(offsets[offsets.len() - 1] + fs::metadata(path)?.len());
        }
        Ok(Self {
            paths: paths.to_vec(),
            offsets,
            position: 0,
            open_volume: None,
        })
    }
}

impl Read for MultiVolumeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // The last volume that starts at or before the position holds it
        let index = self.offsets.partition_point(|&start| start <= self.position) - 1;
        if index >= self.paths.len() {
            return Ok(0);
        }
        let file = match &mut self.open_volume {
            Some((open, file)) if *open == index => file,
            slot => &mut slot.insert((index, fs::File::open(&self.paths[index])?)).1,
        };
        file.seek(SeekFrom::Start(self.position - self.offsets[index]))?;
        let remaining = self.offsets[index + 1] - self.position;
        let len = buf.len().min(usize::try_from(remaining).unwrap_or(usize::MAX));
        let read = file.read(&mut buf[..len])?;
        // A volume that shrank after open must not look like the end of the archive
        if read == 0 && len > 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for MultiVolumeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let total = self.offsets[self.paths.len()];
        let position = match pos {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(offset) => total.checked_add_signed(offset),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
        };
        self.position = position.ok_or(io::ErrorKind::InvalidInput)?;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_seeks_across_volumes() {
        let dir = tempfile::tempdir().unwrap();
        let paths: Vec<PathBuf> = ["abc", "", "defg", "h"]
            .iter()
            .enumerate()
            .map(|(index, data)| {
                let path = dir.path().join(format!("archive.7z.{:03}", index + 1));
                std::fs::write(&path, data).unwrap();
                path
            })
            .collect();
        let mut reader = MultiVolumeReader::open(&paths).unwrap();

        let mut data = String::new();
        reader.read_to_string(&mut data).unwrap();
        assert_eq!(data, "abcdefgh");

        assert_eq!(reader.seek(SeekFrom::End(-5)).unwrap(), 3);
        assert_eq!(reader.seek(SeekFrom::Current(1)).unwrap(), 4);
        let mut buf = [0; 3];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"efg");
        assert!(reader.seek(SeekFrom::Current(-8)).is_err());
        assert_eq!(reader.seek(SeekFrom::Start(20)).unwrap(), 20);
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
    }
}
