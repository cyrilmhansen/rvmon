#![forbid(unsafe_code)]

use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

pub const MAGIC: &[u8; 8] = b"RVSNAP01";
pub const HEADER_LEN: usize = 32;
pub const MAX_WORKSPACE: usize = 0x1_0000;
pub const MAX_DATA: usize = 0x10_0000;
pub const MAX_TRANSPORT_CHUNK: usize = 4096;
pub const METADATA_MAGIC: &[u8; 8] = b"RVMETA01";
pub const MAX_METADATA_SOURCE: usize = 16 * 96;
pub const MAX_METADATA_SYMBOLS: usize = 8;
pub const MAX_METADATA_SYMBOL_NAME: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotImage {
    pub workspace: Vec<u8>,
    pub data: Vec<u8>,
    pub source_lines: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotContext {
    pub x: [u64; 32],
    pub f: [u64; 32],
    pub pc: u64,
    pub fcsr: u32,
    pub mstatus: u64,
    pub mepc: u64,
    pub mcause: u64,
    pub mtval: u64,
}

impl SnapshotContext {
    pub const fn empty() -> Self {
        Self {
            x: [0; 32],
            f: [0xffff_ffff_0000_0000; 32],
            pc: 0,
            fcsr: 0,
            mstatus: 0,
            mepc: 0,
            mcause: 0,
            mtval: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotSymbol {
    pub address: u64,
    pub name: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub context: SnapshotContext,
    pub source: Vec<u8>,
    pub symbols: Vec<SnapshotSymbol>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataError {
    Truncated,
    InvalidMagic,
    UnsupportedVersion(u32),
    InvalidSourceLength(u32),
    InvalidSymbolCount(u32),
    InvalidSymbolNameLength(u16),
    TrailingBytes,
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

#[derive(Debug, PartialEq, Eq)]
pub enum ApplyError {
    Transport(String),
    Protocol(String),
    Format(SnapshotError),
}

pub trait GuestCommandTransport {
    type Error: fmt::Debug;

    fn command(&mut self, command: &str) -> Result<String, Self::Error>;
}

pub trait GuestBinaryCommandTransport: GuestCommandTransport {
    fn command_binary(&mut self, command: &str, payload: &[u8]) -> Result<String, Self::Error>;
}

const GUEST_PROMPT: &[u8] = b"rvmonitor> ";
const GUEST_BINARY_READY: &[u8] = b"snapshot binary ready\r\n";
const MAX_UART_RESPONSE: usize = 2 * 1024 * 1024;

pub struct TcpGuestCommandTransport {
    stream: TcpStream,
}

impl TcpGuestCommandTransport {
    pub fn connect(address: impl ToSocketAddrs) -> io::Result<Self> {
        let stream = TcpStream::connect(address)?;
        stream.set_nodelay(true)?;
        let mut transport = Self { stream };
        transport.read_until_prompt()?;
        Ok(transport)
    }

    pub fn from_stream(stream: TcpStream) -> io::Result<Self> {
        stream.set_nodelay(true)?;
        let mut transport = Self { stream };
        transport.read_until_prompt()?;
        Ok(transport)
    }

    fn read_until_prompt(&mut self) -> io::Result<String> {
        let (response, _) = self.read_until_markers(&[GUEST_PROMPT])?;
        String::from_utf8(response)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "UART is not UTF-8"))
    }

    fn read_until_markers(&mut self, markers: &[&[u8]]) -> io::Result<(Vec<u8>, usize)> {
        let max_marker = markers.iter().map(|marker| marker.len()).max().unwrap_or(0);
        let mut response = Vec::new();
        let mut window = Vec::with_capacity(max_marker);
        let mut byte = [0u8; 1];
        loop {
            self.stream.read_exact(&mut byte)?;
            response.push(byte[0]);
            window.push(byte[0]);
            if window.len() > max_marker {
                window.remove(0);
            }
            for (index, marker) in markers.iter().enumerate() {
                if window.ends_with(marker) {
                    response.truncate(response.len() - marker.len());
                    return Ok((response, index));
                }
            }
            if response.len() > MAX_UART_RESPONSE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "guest UART response exceeds safety limit",
                ));
            }
        }
    }
}

impl GuestCommandTransport for TcpGuestCommandTransport {
    type Error = io::Error;

    fn command(&mut self, command: &str) -> Result<String, Self::Error> {
        self.stream.write_all(command.as_bytes())?;
        self.stream.write_all(b"\n")?;
        self.stream.flush()?;
        self.read_until_prompt()
    }
}

impl GuestBinaryCommandTransport for TcpGuestCommandTransport {
    fn command_binary(&mut self, command: &str, payload: &[u8]) -> Result<String, Self::Error> {
        self.stream.write_all(command.as_bytes())?;
        self.stream.write_all(b"\n")?;
        self.stream.flush()?;
        let (prefix, marker) = self.read_until_markers(&[GUEST_BINARY_READY, GUEST_PROMPT])?;
        if marker == 1 {
            return String::from_utf8(prefix)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "UART is not UTF-8"));
        }
        self.stream.write_all(payload)?;
        self.stream.flush()?;
        let suffix = self.read_until_prompt()?;
        let mut response = String::from_utf8(prefix)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "UART is not UTF-8"))?;
        response.push_str(&suffix);
        Ok(response)
    }
}

pub fn fetch_guest_snapshot<T>(transport: &mut T) -> Result<SnapshotImage, FetchError>
where
    T: GuestCommandTransport,
{
    let manifest_line = transport
        .command("snapshot manifest")
        .map_err(|error| FetchError::Transport(format!("{error:?}")))?;
    let (manifest, chunk_max) = parse_manifest(&manifest_line)?;
    let workspace = fetch_region(
        transport,
        "workspace",
        manifest.workspace_len as usize,
        chunk_max,
    )?;
    let data = fetch_region(transport, "data", manifest.data_len as usize, chunk_max)?;
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

pub fn apply_guest_snapshot<T>(transport: &mut T, image: &SnapshotImage) -> Result<(), ApplyError>
where
    T: GuestBinaryCommandTransport,
{
    image.manifest().map_err(ApplyError::Format)?;
    apply_region(transport, "workspace", &image.workspace)?;
    apply_region(transport, "data", &image.data)?;
    let response = transport
        .command("snapshot restore")
        .map_err(|error| ApplyError::Transport(format!("{error:?}")))?;
    if response.contains("error [") || !response.contains("snapshot restored") {
        return Err(ApplyError::Protocol(
            "guest did not confirm snapshot restore".into(),
        ));
    }
    Ok(())
}

fn apply_region<T: GuestBinaryCommandTransport>(
    transport: &mut T,
    region: &str,
    bytes: &[u8],
) -> Result<(), ApplyError> {
    for (offset, chunk) in bytes.chunks(32).enumerate() {
        let offset = offset * 32;
        let response = transport
            .command_binary(
                &format!("snapshot patchbin {region} {offset} {}", chunk.len()),
                chunk,
            )
            .map_err(|error| ApplyError::Transport(format!("{error:?}")))?;
        if response.contains("error [") || !response.contains("snapshot binary chunk patched") {
            return Err(ApplyError::Protocol(format!(
                "guest rejected snapshot patch {region} offset={offset}"
            )));
        }
    }
    Ok(())
}

fn parse_manifest(line: &str) -> Result<(SnapshotManifest, usize), FetchError> {
    let line = line
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("snapshot-manifest "))
        .ok_or_else(|| FetchError::Protocol("expected snapshot-manifest response".into()))?;
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
    let mut chunk_max = None;
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
            "chunk-max" => {
                chunk_max = Some(parse_decimal_field(key, value)? as usize);
            }
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
    let chunk_max = chunk_max
        .filter(|chunk| (1..=MAX_TRANSPORT_CHUNK).contains(chunk))
        .ok_or_else(|| FetchError::Protocol("manifest has invalid chunk-max".into()))?;
    Ok((manifest, chunk_max))
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
    chunk_max: usize,
) -> Result<Vec<u8>, FetchError> {
    let mut result = Vec::with_capacity(length);
    let mut offset = 0usize;
    while offset < length {
        let chunk_len = (length - offset).min(chunk_max);
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
    let line = line
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("snapshot-chunk "))
        .ok_or_else(|| FetchError::Protocol("expected snapshot-chunk response".into()))?;
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

impl fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("snapshot metadata is truncated"),
            Self::InvalidMagic => formatter.write_str("snapshot metadata magic is invalid"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported snapshot metadata version {version}")
            }
            Self::InvalidSourceLength(length) => {
                write!(
                    formatter,
                    "metadata source length {length} exceeds its limit"
                )
            }
            Self::InvalidSymbolCount(count) => {
                write!(formatter, "metadata symbol count {count} exceeds its limit")
            }
            Self::InvalidSymbolNameLength(length) => {
                write!(
                    formatter,
                    "metadata symbol name length {length} exceeds its limit"
                )
            }
            Self::TrailingBytes => formatter.write_str("snapshot metadata has trailing bytes"),
        }
    }
}

impl std::error::Error for MetadataError {}

impl SnapshotMetadata {
    pub fn encode(&self) -> Result<Vec<u8>, MetadataError> {
        if self.source.len() > MAX_METADATA_SOURCE {
            return Err(MetadataError::InvalidSourceLength(self.source.len() as u32));
        }
        if self.symbols.len() > MAX_METADATA_SYMBOLS {
            return Err(MetadataError::InvalidSymbolCount(self.symbols.len() as u32));
        }
        let mut encoded = Vec::new();
        encoded.extend_from_slice(METADATA_MAGIC);
        encoded.extend_from_slice(&1u32.to_le_bytes());
        for value in self.context.x.iter().chain(self.context.f.iter()) {
            encoded.extend_from_slice(&value.to_le_bytes());
        }
        encoded.extend_from_slice(&self.context.pc.to_le_bytes());
        encoded.extend_from_slice(&self.context.fcsr.to_le_bytes());
        encoded.extend_from_slice(&self.context.mstatus.to_le_bytes());
        encoded.extend_from_slice(&self.context.mepc.to_le_bytes());
        encoded.extend_from_slice(&self.context.mcause.to_le_bytes());
        encoded.extend_from_slice(&self.context.mtval.to_le_bytes());
        encoded.extend_from_slice(&(self.source.len() as u32).to_le_bytes());
        encoded.extend_from_slice(&(self.symbols.len() as u32).to_le_bytes());
        encoded.extend_from_slice(&self.source);
        for symbol in &self.symbols {
            if symbol.name.len() > MAX_METADATA_SYMBOL_NAME {
                return Err(MetadataError::InvalidSymbolNameLength(
                    symbol.name.len() as u16
                ));
            }
            encoded.extend_from_slice(&symbol.address.to_le_bytes());
            encoded.extend_from_slice(&(symbol.name.len() as u16).to_le_bytes());
            encoded.extend_from_slice(&symbol.name);
        }
        Ok(encoded)
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, MetadataError> {
        let mut reader = MetadataReader {
            bytes: encoded,
            offset: 0,
        };
        if reader.take(8)? != METADATA_MAGIC {
            return Err(MetadataError::InvalidMagic);
        }
        let version = reader.u32()?;
        if version != 1 {
            return Err(MetadataError::UnsupportedVersion(version));
        }
        let mut context = SnapshotContext::empty();
        for value in &mut context.x {
            *value = reader.u64()?;
        }
        for value in &mut context.f {
            *value = reader.u64()?;
        }
        context.pc = reader.u64()?;
        context.fcsr = reader.u32()?;
        context.mstatus = reader.u64()?;
        context.mepc = reader.u64()?;
        context.mcause = reader.u64()?;
        context.mtval = reader.u64()?;
        let source_len = reader.u32()?;
        if source_len as usize > MAX_METADATA_SOURCE {
            return Err(MetadataError::InvalidSourceLength(source_len));
        }
        let symbol_count = reader.u32()?;
        if symbol_count as usize > MAX_METADATA_SYMBOLS {
            return Err(MetadataError::InvalidSymbolCount(symbol_count));
        }
        let source = reader.take(source_len as usize)?.to_vec();
        let mut symbols = Vec::with_capacity(symbol_count as usize);
        for _ in 0..symbol_count {
            let address = reader.u64()?;
            let name_len = reader.u16()?;
            if name_len as usize > MAX_METADATA_SYMBOL_NAME {
                return Err(MetadataError::InvalidSymbolNameLength(name_len));
            }
            symbols.push(SnapshotSymbol {
                address,
                name: reader.take(name_len as usize)?.to_vec(),
            });
        }
        if reader.offset != encoded.len() {
            return Err(MetadataError::TrailingBytes);
        }
        Ok(Self {
            context,
            source,
            symbols,
        })
    }
}

struct MetadataReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> MetadataReader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], MetadataError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(MetadataError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(MetadataError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, MetadataError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, MetadataError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, MetadataError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

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
                    "snapshot-manifest format=RVSNAP01 workspace-size={} data-size={} source-lines={} workspace-crc32=0x{:08x} data-crc32=0x{:08x} chunk-max=4096",
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

    struct ApplyGuest {
        patches: Vec<String>,
        restored: bool,
    }

    impl GuestCommandTransport for ApplyGuest {
        type Error = &'static str;

        fn command(&mut self, command: &str) -> Result<String, Self::Error> {
            if command == "snapshot restore" {
                self.restored = true;
                return Ok("snapshot restored (workspace=65536 data=1048576)".into());
            }
            if command.starts_with("snapshot patch ") {
                self.patches.push(command.into());
                return Ok("snapshot chunk patched data offset=0 length=32".into());
            }
            Err("unexpected command")
        }
    }

    impl GuestBinaryCommandTransport for ApplyGuest {
        fn command_binary(
            &mut self,
            command: &str,
            _payload: &[u8],
        ) -> Result<String, Self::Error> {
            self.patches.push(command.into());
            Ok("snapshot binary chunk patched data offset=0 length=32".into())
        }
    }

    #[test]
    fn applies_regions_in_32_byte_patches_before_restore() {
        let image = SnapshotImage {
            workspace: vec![0x11; 33],
            data: vec![0x22; 65],
            source_lines: 0,
        };
        let mut guest = ApplyGuest {
            patches: Vec::new(),
            restored: false,
        };
        apply_guest_snapshot(&mut guest, &image).unwrap();
        assert_eq!(guest.patches.len(), 5);
        assert!(guest.patches[0].starts_with("snapshot patchbin workspace 0 "));
        assert!(guest.patches[2].starts_with("snapshot patchbin data 0 "));
        assert!(guest.restored);
    }

    #[test]
    fn metadata_round_trip_preserves_context_source_and_symbols() {
        let mut context = SnapshotContext::empty();
        context.x[1] = 0x8000_0000;
        context.f[3] = 0x3ff0_0000_0000_0000;
        context.pc = 0x8100_0240;
        context.fcsr = 0x9f;
        let metadata = SnapshotMetadata {
            context,
            source: b"addi x1,x0,1\n".to_vec(),
            symbols: vec![SnapshotSymbol {
                address: 0x8100_0240,
                name: b"entry".to_vec(),
            }],
        };
        let encoded = metadata.encode().unwrap();
        assert_eq!(SnapshotMetadata::decode(&encoded).unwrap(), metadata);
    }

    #[test]
    fn metadata_rejects_corruption_and_limits() {
        let metadata = SnapshotMetadata {
            context: SnapshotContext::empty(),
            source: vec![],
            symbols: vec![],
        };
        let mut encoded = metadata.encode().unwrap();
        encoded[0] ^= 1;
        assert_eq!(
            SnapshotMetadata::decode(&encoded),
            Err(MetadataError::InvalidMagic)
        );
        let oversized = SnapshotMetadata {
            context: SnapshotContext::empty(),
            source: vec![0; MAX_METADATA_SOURCE + 1],
            symbols: vec![],
        };
        assert!(matches!(
            oversized.encode(),
            Err(MetadataError::InvalidSourceLength(_))
        ));
    }
}
