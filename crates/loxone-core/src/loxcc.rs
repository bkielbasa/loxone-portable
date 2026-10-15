use std::io::{Cursor, Read};

use thiserror::Error;

pub const LOXCC_MAGIC: u32 = 0xaabb_ccee;

#[derive(Debug, Error)]
pub enum LoxCcError {
    #[error("LoxCC buffer too short ({0} bytes)")]
    BufferTooShort(usize),
    #[error("Invalid LoxCC header magic: 0x{0:08x} (expected 0xaabbccee)")]
    InvalidMagic(u32),
    #[error("Malformed LoxCC compressed stream: {0}")]
    MalformedStream(&'static str),
    #[error("ZIP archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("I/O error reading archive: {0}")]
    Io(#[from] std::io::Error),
    #[error("No .LoxCC or .Loxone project entry found in archive")]
    MissingProjectEntry,
    #[error("Decompressed payload is not valid UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Decompresses a `.LoxCC` binary buffer (or passes through raw XML) into a UTF-8 `.Loxone` XML string.
pub fn decompress_loxcc(data: &[u8]) -> Result<String, LoxCcError> {
    if is_raw_xml_bytes(data) {
        return Ok(String::from_utf8(data.to_vec())?);
    }

    if data.len() < 16 {
        return Err(LoxCcError::BufferTooShort(data.len()));
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != LOXCC_MAGIC {
        return Err(LoxCcError::InvalidMagic(magic));
    }

    let compressed_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let uncompressed_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    let _checksum = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

    let end = 16usize
        .checked_add(compressed_size)
        .ok_or(LoxCcError::MalformedStream("compressed size overflow"))?;
    if end > data.len() {
        return Err(LoxCcError::BufferTooShort(data.len()));
    }

    let compressed = &data[16..end];
    let mut output = Vec::with_capacity(uncompressed_size.min(64 * 1024 * 1024));
    let mut pos = 0usize;

    while pos < compressed.len() {
        let token = compressed[pos];
        pos += 1;

        let mut lit_len = (token >> 4) as usize;
        let match_nibble = (token & 0x0f) as usize;

        if lit_len == 15 {
            loop {
                if pos >= compressed.len() {
                    return Err(LoxCcError::MalformedStream(
                        "unexpected EOF reading literal length",
                    ));
                }
                let extra = compressed[pos] as usize;
                pos += 1;
                lit_len += extra;
                if extra != 255 {
                    break;
                }
            }
        }

        if pos + lit_len > compressed.len() {
            return Err(LoxCcError::MalformedStream(
                "literal length exceeds compressed buffer",
            ));
        }
        output.extend_from_slice(&compressed[pos..pos + lit_len]);
        pos += lit_len;

        if pos >= compressed.len() {
            break;
        }

        if pos + 2 > compressed.len() {
            return Err(LoxCcError::MalformedStream(
                "unexpected EOF reading match offset",
            ));
        }
        let offset = u16::from_le_bytes([compressed[pos], compressed[pos + 1]]) as usize;
        pos += 2;

        if offset == 0 || offset > output.len() {
            return Err(LoxCcError::MalformedStream("invalid backreference offset"));
        }

        let mut match_len = 4 + match_nibble;
        if match_nibble == 15 {
            loop {
                if pos >= compressed.len() {
                    return Err(LoxCcError::MalformedStream(
                        "unexpected EOF reading match length",
                    ));
                }
                let extra = compressed[pos] as usize;
                pos += 1;
                match_len += extra;
                if extra != 255 {
                    break;
                }
            }
        }

        let match_start = output.len() - offset;
        for i in 0..match_len {
            let b = output[match_start + i];
            output.push(b);
        }
    }

    Ok(String::from_utf8(output)?)
}

/// Extracts and decompresses a `.Loxone` XML string from either a `.zip` backup archive
/// (containing `sps0.LoxCC` or `*.LoxCC` / `*.Loxone`) or a raw `.LoxCC` / `.Loxone` byte buffer.
pub fn extract_loxone_xml(bytes: &[u8]) -> Result<String, LoxCcError> {
    if bytes.starts_with(b"PK\x03\x04") {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
        let mut chosen_index = None;

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_ascii_lowercase();
            let base = name.rsplit('/').next().unwrap_or(&name);
            if base == "sps0.loxcc" {
                chosen_index = Some(i);
                break;
            }
            if chosen_index.is_none() && (base.ends_with(".loxcc") || base.ends_with(".loxone")) {
                chosen_index = Some(i);
            }
        }

        let idx = chosen_index.ok_or(LoxCcError::MissingProjectEntry)?;
        let mut file = archive.by_index(idx)?;
        let mut entry_bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut entry_bytes)?;
        decompress_loxcc(&entry_bytes)
    } else {
        decompress_loxcc(bytes)
    }
}

/// Encodes raw bytes into a valid `.LoxCC` literal-stream frame (`0xaabbccee` header + LZ4 literal block).
pub fn compress_loxcc_literals(raw: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    let len = raw.len();
    if len < 15 {
        payload.push((len as u8) << 4);
    } else {
        payload.push(15u8 << 4);
        let mut rem = len - 15;
        while rem >= 255 {
            payload.push(255);
            rem -= 255;
        }
        payload.push(rem as u8);
    }
    payload.extend_from_slice(raw);

    let mut frame = Vec::with_capacity(16 + payload.len());
    frame.extend_from_slice(&LOXCC_MAGIC.to_le_bytes());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    frame.extend_from_slice(&0u32.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

fn is_raw_xml_bytes(data: &[u8]) -> bool {
    let trimmed = data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(data);
    trimmed.starts_with(b"<?xml")
        || trimmed.starts_with(b"<ControlList")
        || trimmed.starts_with(b"<Document")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    #[test]
    fn test_decompress_loxcc_literals_and_backreferences() {
        // Construct an LZ4 block with a literal "ABCD" followed by an offset=4, match_len=8 backreference -> "ABCDABCDABCD"
        let mut payload = Vec::new();
        // token: lit_len = 4 (high nibble 4), match_nibble = 4 (match_len = 4 + 4 = 8)
        payload.push((4 << 4) | 4);
        payload.extend_from_slice(b"ABCD");
        // offset = 4 (u16_le)
        payload.extend_from_slice(&4u16.to_le_bytes());

        let mut frame = Vec::new();
        frame.extend_from_slice(&LOXCC_MAGIC.to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(&12u32.to_le_bytes());
        frame.extend_from_slice(&0u32.to_le_bytes());
        frame.extend_from_slice(&payload);

        let decompressed = decompress_loxcc(&frame).expect("should decompress backreference");
        assert_eq!(decompressed, "ABCDABCDABCD");
    }

    #[test]
    fn test_compress_and_extract_loxcc_inside_zip_archive() {
        let xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\
            <ControlList Version=\"274\">\
                <C Type=\"Document\" U=\"doc-zip\" Title=\"ZipProject\" ConfigVersion=\"17020828\"/>\
            </ControlList>";
        let loxcc_bytes = compress_loxcc_literals(xml.as_bytes());

        let mut zip_buf = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut zip_buf);
            writer
                .start_file("sps0.LoxCC", SimpleFileOptions::default())
                .unwrap();
            writer.write_all(&loxcc_bytes).unwrap();
            writer.finish().unwrap();
        }

        let archive_bytes = zip_buf.into_inner();
        let extracted_xml =
            extract_loxone_xml(&archive_bytes).expect("should unpack sps0.LoxCC from zip");
        assert_eq!(extracted_xml, xml);

        let parsed =
            crate::parser::parse_loxone_xml(&extracted_xml).expect("extracted XML should parse");
        assert_eq!(parsed.metadata.title, "ZipProject");
    }

    #[test]
    fn test_invalid_loxcc_magic_returns_error() {
        let mut bad = vec![0u8; 20];
        bad[0..4].copy_from_slice(&0x12345678u32.to_le_bytes());
        let err = decompress_loxcc(&bad).unwrap_err();
        assert!(matches!(err, LoxCcError::InvalidMagic(0x12345678)));
    }
}
