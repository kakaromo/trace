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
/// Uses lossy decoding to handle invalid UTF-8 sequences.
pub fn read_to_string_auto(filepath: &str) -> io::Result<String> {
    let mut file = File::open(filepath)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    // Detect encoding from the file content
    let encoding = detect_encoding(&bytes);

    // Decode with lossy conversion (invalid characters are replaced)
    let (decoded, _encoding_used, _had_errors) = encoding.decode(&bytes);

    Ok(decoded.into_owned())
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