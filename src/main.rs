use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::mpsc::channel;
use std::thread;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        return Ok(());
    }
    let input_file_path = &args[1];
    let output_file_path = &args[2];

    let input_file = match File::open(input_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return Ok(());
        }
    };

    let mut output_file = match File::create(output_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            return Ok(());
        }
    };

    let (tx, rx) = channel();
    let chunk_size = 1024;
    let num_threads = 4;

    // Spawn multiple threads to read and compress chunks of data
    for _ in 0..num_threads {
        let tx = tx.clone();
        let mut buffer = vec![0; chunk_size];
        let mut input_file_clone = input_file.try_clone().unwrap();
        thread::spawn(move || {
            let mut compressor = DeflateEncoder::new(Vec::new(), Compression::best());
            loop {
                match input_file_clone.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(bytes_read) => {
                        if let Err(e) = compressor.write_all(&buffer[..bytes_read]) {
                            tx.send(Err(e)).unwrap();
                            return;
                        }
                        if compressor.get_ref().len() >= chunk_size {
                            let compressed_data = compressor.finish().unwrap();
                            tx.send(Ok(compressed_data)).unwrap();
                            compressor = DeflateEncoder::new(Vec::new(), Compression::best());
                        }
                    }
                    Err(e) => {
                        tx.send(Err(e)).unwrap();
                        return;
                    }
                }
            }
            let compressed_data = compressor.finish().unwrap();
            tx.send(Ok(compressed_data)).unwrap();
        });
    }

    // Collect compressed chunks from threads and write them to the output file
    let mut compressed_data = Vec::new();
    for _ in 0..num_threads {
        match rx.recv() {
            Ok(Ok(data)) => compressed_data.push(data),
            Ok(Err(e)) => {
                eprintln!("Failed to compress data: {}", e);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Failed to receive compressed data: {}", e);
                return Ok(());
            }
        }
    }
    compressed_data.sort_by_key(|chunk| chunk.len());
    for chunk in compressed_data {
        if let Err(e) = output_file.write_all(&chunk) {
            eprintln!("Failed to write compressed data to output file: {}", e);
            return Ok(());
        }
    }

    Ok(())
}
