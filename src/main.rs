use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        return Ok(());
    }
    let input_file_path = &args[1];
    let output_file_path = &args[2];

    let mut input_file = match File::open(input_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return Ok(());
        }
    };

    let output_file = match File::create(output_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            return Ok(());
        }
    };

    let mut compressor = DeflateEncoder::new(output_file, Compression::best());
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = match input_file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                eprintln!("Failed to read from input file: {}", e);
                return Ok(());
            }
        };
        if let Err(e) = compressor.write_all(&buffer[..bytes_read]) {
            eprintln!("Failed to write to compressor: {}", e);
            return Ok(());
        }
    }
    if let Err(e) = compressor.finish() {
        eprintln!("Failed to finish compressor: {}", e);
        return Ok(());
    }
    Ok(())
}
