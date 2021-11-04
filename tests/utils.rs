use std::io::Write;

use rand::RngCore;

#[macro_export]
macro_rules! ouch {
    ($($e:expr),*) => {
        ::assert_cmd::Command::cargo_bin("ouch")
            .expect("Failed to find ouch executable")
            $(.arg($e))*
            .unwrap();
    }
}

pub fn create_file_random(file: &mut impl Write, rng: &mut impl RngCore) {
    let data = &mut Vec::with_capacity((rng.next_u32() % 8192) as usize);
    rng.fill_bytes(data);
    file.write_all(data).unwrap();
}
