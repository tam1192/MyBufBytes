use std::fs::File;
use my_buf_bytes::BufBytes;

fn main() {
    let mut file = File::open("Cargo.toml").expect("Failed to open file");
    let buf_bytes = BufBytes::new(&mut file).unwrap();
    for byte in buf_bytes {
        print!("{}", byte as char);
    }
}