use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::env;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

struct Cli {
    input_file_path: String,
    output_file_path: String,
}

impl Cli {
    fn new(input_file_path: String, output_file_path: String) -> Self {
        Cli {
            input_file_path,
            output_file_path,
        }
    }
}

struct Chunk {
    compressed_data: Option<Vec<u8>>,
}

// An error that occurred during compression
#[derive(Debug)]
pub enum CompressionError {
    // define the variants of the error
    InvalidData,
    IOError(std::io::Error),
}

impl Error for CompressionError {}

impl Display for CompressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionError::InvalidData => {
                write!(f, "Invalid data")
            }
            CompressionError::IOError(e) => {
                write!(f, "I/O error: {}", e)
            }
        }
    }
}

impl From<std::io::Error> for CompressionError {
    fn from(err: std::io::Error) -> Self {
        CompressionError::IOError(err)
    }
}

// A message that can be sent through a channel
enum CompressionMessage {
    Data(Vec<u8>),
    Error(CompressionError),
    Done,
}

// A worker thread responsible for compressing data
struct CompressionWorker {
    sender: Sender<CompressionMessage>,
    chunk_size: usize,
}

impl CompressionWorker {
    fn new(sender: Sender<CompressionMessage>, chunk_size: usize) -> Self {
        CompressionWorker { sender, chunk_size }
    }

    fn run(&self, mut input_file: File) {
        let mut buffer = vec![0; self.chunk_size];
        let mut compressor = DeflateEncoder::new(Vec::new(), Compression::best());

        loop {
            match input_file.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    if let Err(e) = compressor.write_all(&buffer[..bytes_read]) {
                        self.sender
                            .send(CompressionMessage::Error(CompressionError::IOError(e)))
                            .unwrap();
                        return;
                    }
                    if compressor.get_ref().len() >= self.chunk_size {
                        let compressed_data = compressor.finish().unwrap();
                        self.sender
                            .send(CompressionMessage::Data(compressed_data))
                            .unwrap();
                        compressor = DeflateEncoder::new(Vec::new(), Compression::best());
                    }
                }
                Err(e) => {
                    self.sender
                        .send(CompressionMessage::Error(CompressionError::IOError(e)))
                        .unwrap();
                    return;
                }
            }
        }

        let compressed_data = compressor.finish().unwrap();
        self.sender
            .send(CompressionMessage::Data(compressed_data))
            .unwrap();
        self.sender.send(CompressionMessage::Done).unwrap();
    }
}

struct Compressor {
    chunk_size: usize,
    num_threads: usize,
}

impl Compressor {
    fn new(chunk_size: usize, num_threads: usize) -> Self {
        Compressor {
            chunk_size,
            num_threads,
        }
    }

    fn compress(&self, input_file: &mut File) -> Result<Vec<Chunk>, CompressionError> {
        let (tx, rx): (Sender<CompressionMessage>, Receiver<CompressionMessage>) = channel();

        // Spawn multiple threads to read and compress chunks of data
        let mut threads = Vec::new();
        for _ in 0..self.num_threads {
            let tx = tx.clone();
            let chunk_size = self.chunk_size;
            let input_file_clone = input_file.try_clone().unwrap();

            let worker = CompressionWorker::new(tx, chunk_size);
            let thread = thread::spawn(move || {
                worker.run(input_file_clone);
            });

            threads.push(thread);
        }

        // Collect compressed chunks from threads
        let mut chunks = Vec::new();
        for _ in 0..self.num_threads {
            match rx.recv() {
                Ok(CompressionMessage::Data(data)) => {
                    chunks.push(Chunk {
                        compressed_data: Some(data),
                    });
                }
                Ok(CompressionMessage::Error(e)) => {
                    eprintln!("Failed to compress data: {:?}", e);
                    return Err(e);
                }
                Ok(CompressionMessage::Done) => {
                    // The CompressionWorker has finished compressing all the data
                    break;
                }
                Err(e) => {
                    eprintln!("Failed to receive compressed data: {:?}", e);
                }
            }
        }

        // Wait for all threads to finish
        for thread in threads {
            thread.join().unwrap();
        }

        // Return compressed chunks
        Ok(chunks)
    }
}

struct Writer {}

impl Writer {
    fn write(chunks: &[Chunk], output_file: &mut File) -> io::Result<()> {
        let mut compressed_data: Vec<&[u8]> = chunks
            .iter()
            .filter_map(|chunk| chunk.compressed_data.as_ref().map(|d| d.as_slice()))
            .collect();
        compressed_data.sort_by_key(|chunk| chunk.len());

        for chunk in compressed_data {
            if let Err(e) = output_file.write_all(chunk) {
                eprintln!("Failed to write compressed data to output file: {}", e);
                return Err(e);
            }
        }
        Ok(())
    }
}

fn get_args() -> Result<Cli, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!();
        return Err(format!("Usage: {} <input_file> <output_file>", args[0]).into());
    }

    Ok(Cli::new(args[1].to_owned(), args[2].to_owned()))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = get_args()?;
    let mut input_file = match File::open(args.input_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return Ok(());
        }
    };

    let mut output_file = match File::create(args.output_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            return Ok(());
        }
    };

    let chunk_size = 1024;
    let num_threads = 4;
    let compressor = Compressor::new(chunk_size, num_threads);

    let compressed_data = compressor.compress(&mut input_file)?;

    Writer::write(&compressed_data, &mut output_file)?;

    Ok(())
}
