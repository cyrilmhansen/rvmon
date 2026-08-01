#![forbid(unsafe_code)]

use std::fmt;

pub const MAGIC: &[u8; 8] = b"RVSNAP01";
pub const HEADER_LEN: usize = 32;
pub const MAX_WORKSPACE: usize = 0x1_0000;
pub const MAX_DATA: usize = 0x10_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotImage {
    pub workspace: Vec<u8>,
    pub data: Vec<u8>,
    pub source_lines: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotManifest {
    pub workspace_len: u32,
    pub data_len: u32,
    pub source_lines: u32,
    pub workspace_crc32: u32,
    pub data_crc32: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotError {
    Truncated,
    InvalidMagic,
    InvalidHeaderLength(u32),
    InvalidRegionLength { region: &'static str, length: u32 },
    LengthOverflow,
    TrailingBytes,
    ChecksumMismatch { region: &'static str },
}

#[derive(Debug, PartialEq, Eq)]
pub enum FetchError {
    Transport(String),
    Protocol(String),
    Format(SnapshotError),
}

pub trait GuestCommandTransport {
    type Error: fmt::Debug;

    fn command(&mut self, command: &str) -> Result<String, Self::Error>;
}

pub fn fetch_guest_snapshot<T>(transport: &mut T) -> Result<SnapshotImage, FetchError>
where
    T: GuestCommandTransport,
{
    let manifest_line = transport
        .command("snapshot manifest")
        .map_err(|error| FetchError::Transport(format!("{error:?}")))?;
    let manifest = parse_manifest(&manifest_line)?;
    let workspace = fetch_region(transport, "workspace", manifest.workspace_len as usize)?;
    let data = fetch_region(transport, "data", manifest.data_len as usize)?;
    let image = SnapshotImage {
        workspace,
        data,
        source_lines: manifest.source_lines,
    };
    let actual = image.manifest().map_err(FetchError::Format)?;
    if actual.workspace_crc32 != manifest.workspace_crc32 {
        return Err(FetchError::Protocol(
            "workspace CRC-32 differs from guest manifest".into(),
        ));
    }
    if actual.data_crc32 != manifest.data_crc32 {
        return Err(FetchError::Protocol(
            "data CRC-32 differs from guest manifest".into(),
        ));
    }
    Ok(image)
}

fn parse_manifest(line: &str) -> Result<SnapshotManifest, FetchError> {
    let mut fields = line.split_whitespace();
    if fields.next() != Some("snapshot-manifest") {
        return Err(FetchError::Protocol(
            "expected snapshot-manifest response".into(),
        ));
    }
    let mut format = None;
    let mut workspace_len = None;
    let mut data_len = None;
    let mut source_lines = None;
    let mut workspace_crc32 = None;
    let mut data_crc32 = None;
    for field in fields {
        let (key, value) = field
            .split_once('=')
            .ok_or_else(|| FetchError::Protocol(format!("invalid manifest field {field}")))?;
        match key {
            "format" => format = Some(value),
            "workspace-size" => workspace_len = Some(parse_decimal_field(key, value)?),
            "data-size" => data_len = Some(parse_decimal_field(key, value)?),
            "source-lines" => source_lines = Some(parse_decimal_field(key, value)?),
            "workspace-crc32" => workspace_crc32 = Some(parse_hex_field(key, value)?),
            "data-crc32" => data_crc32 = Some(parse_hex_field(key, value)?),
            "chunk-max" if value == "32" => {}
            _ => {}
        }
    }
    if format != Some("RVSNAP01") {
        return Err(FetchError::Protocol("unsupported snapshot format".into()));
    }
    let manifest = SnapshotManifest {
        workspace_len: workspace_len
            .ok_or_else(|| FetchError::Protocol("manifest lacks workspace-size".into()))?,
        data_len: data_len
            .ok_or_else(|| FetchError::Protocol("manifest lacks data-size".into()))?,
        source_lines: source_lines
            .ok_or_else(|| FetchError::Protocol("manifest lacks source-lines".into()))?,
        workspace_crc32: workspace_crc32
            .ok_or_else(|| FetchError::Protocol("manifest lacks workspace-crc32".into()))?,
        data_crc32: data_crc32
            .ok_or_else(|| FetchError::Protocol("manifest lacks data-crc32".into()))?,
    };
    validate_lengths(manifest.workspace_len as usize, manifest.data_len as usize)
        .map_err(FetchError::Format)?;
    Ok(manifest)
}

fn parse_decimal_field(key: &str, value: &str) -> Result<u32, FetchError> {
    value
        .parse()
        .map_err(|_| FetchError::Protocol(format!("invalid decimal {key}={value}")))
}

fn parse_hex_field(key: &str, value: &str) -> Result<u32, FetchError> {
    value
        .strip_prefix("0x")
        .and_then(|value| u32::from_str_radix(value, 16).ok())
        .ok_or_else(|| FetchError::Protocol(format!("invalid hexadecimal {key}={value}")))
}

fn fetch_region<T: GuestCommandTransport>(
    transport: &mut T,
    region: &str,
    length: usize,
) -> Result<Vec<u8>, FetchError> {
    let mut result = Vec::with_capacity(length);
    let mut offset = 0usize;
    while offset < length {
        let chunk_len = (length - offset).min(32);
        let response = transport
            .command(&format!("snapshot dump {region} {offset} {chunk_len}"))
            .map_err(|error| FetchError::Transport(format!("{error:?}")))?;
        let bytes = parse_chunk(&response, region, offset, chunk_len)?;
        result.extend_from_slice(&bytes);
        offset += chunk_len;
    }
    Ok(result)
}

fn parse_chunk(
    line: &str,
    expected_region: &str,
    expected_offset: usize,
    expected_length: usize,
) -> Result<Vec<u8>, FetchError> {
    let mut fields = line.split_whitespace();
    if fields.next() != Some("snapshot-chunk") {
        return Err(FetchError::Protocol(
            "expected snapshot-chunk response".into(),
        ));
    }
    let region = fields
        .next()
        .ok_or_else(|| FetchError::Protocol("chunk lacks region".into()))?;
    let offset = field_value(&mut fields, "offset")?;
    let length = field_value(&mut fields, "length")?;
    let hex = field_text(&mut fields, "hex")?;
    if region != expected_region || offset != expected_offset || length != expected_length {
        return Err(FetchError::Protocol(format!(
            "unexpected chunk {region} offset={offset} length={length}"
        )));
    }
    if hex.len() != length * 2 {
        return Err(FetchError::Protocol(
            "chunk hex length does not match".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(length);
    for pair in hex.as_bytes().chunks_exact(2) {
        let high = hex_nibble(pair[0]);
        let low = hex_nibble(pair[1]);
        let (Some(high), Some(low)) = (high, low) else {
            return Err(FetchError::Protocol("chunk contains non-hex data".into()));
        };
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn field_value<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    key: &str,
) -> Result<usize, FetchError> {
    let value = fields
        .next()
        .and_then(|field| field.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| FetchError::Protocol(format!("chunk lacks {key}")))?;
    value
        .parse()
        .map_err(|_| FetchError::Protocol(format!("invalid chunk {key}={value}")))
}

fn field_text<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    key: &str,
) -> Result<&'a str, FetchError> {
    fields
        .next()
        .and_then(|field| field.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| FetchError::Protocol(format!("chunk lacks {key}")))
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("snapshot is truncated"),
            Self::InvalidMagic => formatter.write_str("snapshot magic is not RVSNAP01"),
            Self::InvalidHeaderLength(length) => {
                write!(formatter, "unsupported snapshot header length {length}")
            }
            Self::InvalidRegionLength { region, length } => {
                write!(
                    formatter,
                    "{region} region length {length} is outside the guest contract"
                )
            }
            Self::LengthOverflow => formatter.write_str("snapshot length overflows host size"),
            Self::TrailingBytes => formatter.write_str("snapshot contains trailing bytes"),
            Self::ChecksumMismatch { region } => {
                write!(
                    formatter,
                    "snapshot {region} CRC-32 does not match its manifest"
                )
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

impl SnapshotImage {
    pub fn manifest(&self) -> Result<SnapshotManifest, SnapshotError> {
        validate_lengths(self.workspace.len(), self.data.len())?;
        Ok(SnapshotManifest {
            workspace_len: self.workspace.len() as u32,
            data_len: self.data.len() as u32,
            source_lines: self.source_lines,
            workspace_crc32: crc32(&self.workspace),
            data_crc32: crc32(&self.data),
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, SnapshotError> {
        let manifest = self.manifest()?;
        let total = HEADER_LEN
            .checked_add(self.workspace.len())
            .and_then(|length| length.checked_add(self.data.len()))
            .ok_or(SnapshotError::LengthOverflow)?;
        let mut encoded = Vec::with_capacity(total);
        encoded.extend_from_slice(MAGIC);
        encoded.extend_from_slice(&(HEADER_LEN as u32).to_le_bytes());
        encoded.extend_from_slice(&manifest.workspace_len.to_le_bytes());
        encoded.extend_from_slice(&manifest.data_len.to_le_bytes());
        encoded.extend_from_slice(&manifest.source_lines.to_le_bytes());
        encoded.extend_from_slice(&manifest.workspace_crc32.to_le_bytes());
        encoded.extend_from_slice(&manifest.data_crc32.to_le_bytes());
        encoded.extend_from_slice(&self.workspace);
        encoded.extend_from_slice(&self.data);
        Ok(encoded)
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, SnapshotError> {
        if encoded.len() < HEADER_LEN {
            return Err(SnapshotError::Truncated);
        }
        if &encoded[..MAGIC.len()] != MAGIC {
            return Err(SnapshotError::InvalidMagic);
        }
        let header_len = read_u32(encoded, 8)?;
        if header_len as usize != HEADER_LEN {
            return Err(SnapshotError::InvalidHeaderLength(header_len));
        }
        let workspace_len = read_u32(encoded, 12)?;
        let data_len = read_u32(encoded, 16)?;
        validate_lengths(workspace_len as usize, data_len as usize)?;
        let source_lines = read_u32(encoded, 20)?;
        let workspace_crc32 = read_u32(encoded, 24)?;
        let data_crc32 = read_u32(encoded, 28)?;
        let workspace_start = HEADER_LEN;
        let data_start = workspace_start
            .checked_add(workspace_len as usize)
            .ok_or(SnapshotError::LengthOverflow)?;
        let end = data_start
            .checked_add(data_len as usize)
            .ok_or(SnapshotError::LengthOverflow)?;
        if encoded.len() < end {
            return Err(SnapshotError::Truncated);
        }
        if encoded.len() != end {
            return Err(SnapshotError::TrailingBytes);
        }
        let workspace = encoded[workspace_start..data_start].to_vec();
        let data = encoded[data_start..end].to_vec();
        if crc32(&workspace) != workspace_crc32 {
            return Err(SnapshotError::ChecksumMismatch {
                region: "workspace",
            });
        }
        if crc32(&data) != data_crc32 {
            return Err(SnapshotError::ChecksumMismatch { region: "data" });
        }
        Ok(Self {
            workspace,
            data,
            source_lines,
        })
    }
}

fn validate_lengths(workspace: usize, data: usize) -> Result<(), SnapshotError> {
    if workspace > MAX_WORKSPACE {
        return Err(SnapshotError::InvalidRegionLength {
            region: "workspace",
            length: workspace as u32,
        });
    }
    if data > MAX_DATA {
        return Err(SnapshotError::InvalidRegionLength {
            region: "data",
            length: data as u32,
        });
    }
    Ok(())
}

fn read_u32(bytes: &[u8], start: usize) -> Result<u32, SnapshotError> {
    let end = start.checked_add(4).ok_or(SnapshotError::LengthOverflow)?;
    let value = bytes.get(start..end).ok_or(SnapshotError::Truncated)?;
    Ok(u32::from_le_bytes(
        value.try_into().expect("length checked"),
    ))
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_deterministic() {
        let image = SnapshotImage {
            workspace: vec![0x11, 0x22],
            data: vec![0xaa, 0xbb, 0xcc],
            source_lines: 7,
        };
        let encoded = image.encode().unwrap();
        assert_eq!(encoded, image.encode().unwrap());
        assert_eq!(SnapshotImage::decode(&encoded).unwrap(), image);
    }

    #[test]
    fn rejects_data_corruption() {
        let image = SnapshotImage {
            workspace: vec![0; 4],
            data: vec![1, 2, 3, 4],
            source_lines: 0,
        };
        let mut encoded = image.encode().unwrap();
        *encoded.last_mut().unwrap() ^= 1;
        assert_eq!(
            SnapshotImage::decode(&encoded),
            Err(SnapshotError::ChecksumMismatch { region: "data" })
        );
    }

    #[test]
    fn rejects_trailing_bytes_and_oversized_regions() {
        let image = SnapshotImage {
            workspace: vec![],
            data: vec![],
            source_lines: 0,
        };
        let mut encoded = image.encode().unwrap();
        encoded.push(0);
        assert_eq!(
            SnapshotImage::decode(&encoded),
            Err(SnapshotError::TrailingBytes)
        );
        let oversized = SnapshotImage {
            workspace: vec![0; MAX_WORKSPACE + 1],
            data: vec![],
            source_lines: 0,
        };
        assert!(matches!(
            oversized.encode(),
            Err(SnapshotError::InvalidRegionLength {
                region: "workspace",
                ..
            })
        ));
    }

    struct FakeGuest {
        image: SnapshotImage,
        corrupt_data: bool,
    }

    impl GuestCommandTransport for FakeGuest {
        type Error = &'static str;

        fn command(&mut self, command: &str) -> Result<String, Self::Error> {
            let manifest = self.image.manifest().unwrap();
            if command == "snapshot manifest" {
                return Ok(format!(
                    "snapshot-manifest format=RVSNAP01 workspace-size={} data-size={} source-lines={} workspace-crc32=0x{:08x} data-crc32=0x{:08x} chunk-max=32",
                    manifest.workspace_len,
                    manifest.data_len,
                    manifest.source_lines,
                    manifest.workspace_crc32,
                    manifest.data_crc32
                ));
            }
            let mut fields = command.split_whitespace();
            if fields.next() != Some("snapshot") || fields.next() != Some("dump") {
                return Err("unexpected command");
            }
            let region = fields.next().ok_or("missing region")?;
            let offset: usize = fields.next().ok_or("missing offset")?.parse().unwrap();
            let length: usize = fields.next().ok_or("missing length")?.parse().unwrap();
            let source = if region == "workspace" {
                &self.image.workspace
            } else {
                &self.image.data
            };
            let mut bytes = source[offset..offset + length].to_vec();
            if self.corrupt_data && region == "data" && offset == 0 {
                bytes[0] ^= 1;
            }
            let hex = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            Ok(format!(
                "snapshot-chunk {region} offset={offset} length={length} hex={hex}"
            ))
        }
    }

    #[test]
    fn fetches_chunks_and_checks_manifest_crc() {
        let image = SnapshotImage {
            workspace: vec![0x11; 65],
            data: vec![0x22; 67],
            source_lines: 3,
        };
        let mut guest = FakeGuest {
            image: image.clone(),
            corrupt_data: false,
        };
        assert_eq!(fetch_guest_snapshot(&mut guest).unwrap(), image);

        guest.corrupt_data = true;
        assert_eq!(
            fetch_guest_snapshot(&mut guest),
            Err(FetchError::Protocol(
                "data CRC-32 differs from guest manifest".into()
            ))
        );
    }
}
