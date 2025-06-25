use std::fs::File;
use std::io::{self, BufReader, Read};
use encoding_rs_io::DecodeReaderBytesBuilder;
use chardetng::EncodingDetector;
use encoding_rs::Encoding;

pub type EncodedBufReader = BufReader<encoding_rs_io::DecodeReaderBytes<File, Vec<u8>>>;

/// Open a file with automatic encoding detection and return a buffered reader
/// that yields UTF-8 text.
pub fn open_encoded_reader(filepath: &str, buffer_size: usize) -> io::Result<EncodedBufReader> {
    let file = File::open(filepath)?;
    let decoder = DecodeReaderBytesBuilder::new()
        .encoding(None)
        .build(file);
    Ok(BufReader::with_capacity(buffer_size, decoder))
}

/// Read the entire file with automatic encoding detection into a UTF-8 `String`.
pub fn read_to_string_auto(filepath: &str) -> io::Result<String> {
    let file = File::open(filepath)?;
    let mut decoder = DecodeReaderBytesBuilder::new()
        .encoding(None)
        .build(file);
    let mut contents = String::new();
    decoder.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Detect text encoding from a byte slice.
pub fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    detector.guess(None, true)
}

/// Decode a byte slice to a UTF-8 `String` using automatic encoding detection.
pub fn decode_bytes_auto(bytes: &[u8]) -> String {
    let enc = detect_encoding(bytes);
    let (cow, _, _) = enc.decode(bytes);
    cow.into_owned()
}