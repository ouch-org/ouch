use std::cmp;

const UNIT_PREFIXES: [&str; 6] = ["", "k", "M", "G", "T", "P"];

pub struct Bytes {
    bytes: f64,
}

impl Bytes {
    pub fn new(bytes: u64) -> Self {
        Self {
            bytes: bytes as f64,
        }
    }
}

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num = self.bytes;
        debug_assert!(num >= 0.0);
        if num < 1_f64 {
            return write!(f, "{} B", num);
        }
        let delimiter = 1000_f64;
        let exponent = cmp::min((num.ln() / 6.90775).floor() as i32, 4);

        write!(f, "{:.2} ", num / delimiter.powi(exponent))?;
        write!(f, "{}B", UNIT_PREFIXES[exponent as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_formatting() {
        fn format_bytes(bytes: u64) -> String {
            format!("{}", Bytes::new(bytes))
        }
        let b = 1;
        let kb = b * 1000;
        let mb = kb * 1000;
        let gb = mb * 1000;

        assert_eq!("0 B", format_bytes(0)); // This is weird
        assert_eq!("1.00 B", format_bytes(b));
        assert_eq!("999.00 B", format_bytes(b * 999));
        assert_eq!("12.00 MB", format_bytes(mb * 12));
        assert_eq!("123.00 MB", format_bytes(mb * 123));
        assert_eq!("5.50 MB", format_bytes(mb * 5 + kb * 500));
        assert_eq!("7.54 GB", format_bytes(gb * 7 + 540 * mb));
        assert_eq!("1.20 TB", format_bytes(gb * 1200));

        // bytes
        assert_eq!("234.00 B", format_bytes(234));
        assert_eq!("999.00 B", format_bytes(999));
        // kilobytes
        assert_eq!("2.23 kB", format_bytes(2234));
        assert_eq!("62.50 kB", format_bytes(62500));
        assert_eq!("329.99 kB", format_bytes(329990));
        // megabytes
        assert_eq!("2.75 MB", format_bytes(2750000));
        assert_eq!("55.00 MB", format_bytes(55000000));
        assert_eq!("987.65 MB", format_bytes(987654321));
        // gigabytes
        assert_eq!("5.28 GB", format_bytes(5280000000));
        assert_eq!("95.20 GB", format_bytes(95200000000));
        assert_eq!("302.00 GB", format_bytes(302000000000));
    }
}
