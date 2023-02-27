use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let mut input_file = File::open("test.txt")?;
    let mut output_file = File::create("output.deflate")?;

    let mut compressor = DeflateEncoder::new(&mut output_file, Compression::best());
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = input_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        compressor.write_all(&buffer[..bytes_read])?;
    }
    compressor.finish()?;
    Ok(())
}
