#![forbid(unsafe_code)]

use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

use luna_target_api::{
    ExecutionOutcome, StopEvent, StopReason, TargetBackend, TargetCapabilities, TargetContext,
};

const MAX_PACKET: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Riscv64RegisterLayout {
    pub x_start: usize,
    pub f_start: usize,
    pub pc_index: usize,
    pub register_width: usize,
}

impl Riscv64RegisterLayout {
    pub const QEMU_DEFAULT: Self = Self {
        x_start: 0,
        f_start: 32,
        pc_index: 64,
        register_width: 8,
    };

    /// Layout observed from QEMU's integer-only RV64 GDB target description.
    pub const QEMU_INTEGER: Self = Self {
        x_start: 0,
        f_start: usize::MAX,
        pc_index: 32,
        register_width: 8,
    };

    fn has_float_registers(self) -> bool {
        self.f_start != usize::MAX
    }

    fn packet_length(self) -> usize {
        let highest_register = if self.has_float_registers() {
            self.pc_index.max(self.f_start + 31)
        } else {
            self.pc_index
        };
        (highest_register + 1) * self.register_width * 2
    }
}

#[derive(Debug)]
pub enum QemuError {
    Io(io::Error),
    Protocol(&'static str),
    Message(String),
}

impl fmt::Display for QemuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Protocol(message) => write!(formatter, "GDB RSP protocol error: {message}"),
            Self::Message(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for QemuError {}

impl From<io::Error> for QemuError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct GdbRemote<S> {
    stream: S,
    context: TargetContext,
    layout: Riscv64RegisterLayout,
    capabilities: TargetCapabilities,
    instruction_count: u64,
}

impl<S: Read + Write> GdbRemote<S> {
    pub fn new(stream: S) -> Self {
        Self::with_layout(stream, Riscv64RegisterLayout::QEMU_DEFAULT)
    }

    pub fn with_layout(stream: S, layout: Riscv64RegisterLayout) -> Self {
        let capabilities = if layout.has_float_registers() {
            TargetCapabilities::RV64_BARE_METAL_V1
        } else {
            TargetCapabilities::RV64_INTEGER_BARE_METAL_V1
        };
        Self {
            stream,
            context: TargetContext::empty(),
            layout,
            capabilities,
            instruction_count: 0,
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    pub fn context_from_g_packet(
        packet: &[u8],
        layout: Riscv64RegisterLayout,
    ) -> Result<TargetContext, QemuError> {
        if packet.len() < layout.packet_length() || layout.register_width != 8 {
            return Err(QemuError::Message(format!(
                "unsupported or truncated g packet: {} bytes, need at least {}",
                packet.len(),
                layout.packet_length()
            )));
        }
        let register = |index: usize| -> Result<u64, QemuError> {
            let start = index
                .checked_mul(layout.register_width)
                .ok_or(QemuError::Protocol("register offset overflow"))?;
            let end = start + layout.register_width;
            let bytes = decode_hex(&packet[start * 2..end * 2])?;
            Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
        };
        let mut context = TargetContext::empty();
        for index in 0..32 {
            context.x[index] = register(layout.x_start + index)?;
            if layout.has_float_registers() {
                context.f[index] = register(layout.f_start + index)?;
            }
        }
        context.pc = register(layout.pc_index)?;
        context.mepc = context.pc;
        Ok(context)
    }

    fn transact(&mut self, payload: &[u8]) -> Result<Vec<u8>, QemuError> {
        if payload.len() > MAX_PACKET {
            return Err(QemuError::Protocol("outgoing packet is too large"));
        }
        self.stream.write_all(b"$")?;
        self.stream.write_all(payload)?;
        self.stream.write_all(b"#")?;
        let checksum = payload
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        self.stream
            .write_all(format!("{checksum:02x}").as_bytes())?;
        self.stream.flush()?;

        let acknowledgment = self.read_byte()?;
        if acknowledgment == b'-' {
            return Err(QemuError::Protocol("target rejected packet"));
        }
        let mut byte = acknowledgment;
        while byte != b'$' {
            byte = self.read_byte()?;
        }
        let response = self.read_packet_body()?;
        self.acknowledge_packet()?;
        Ok(response)
    }

    fn read_packet_body(&mut self) -> Result<Vec<u8>, QemuError> {
        let mut response = Vec::new();
        let mut byte;
        loop {
            byte = self.read_byte()?;
            match byte {
                b'#' => break,
                b'}' => response.push(self.read_byte()? ^ 0x20),
                _ => response.push(byte),
            }
            if response.len() > MAX_PACKET {
                return Err(QemuError::Protocol("incoming packet is too large"));
            }
        }
        let high = self.read_byte()?;
        let low = self.read_byte()?;
        let received = (hex_digit(high)? << 4) | hex_digit(low)?;
        let expected = response
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        if received != expected {
            return Err(QemuError::Protocol("packet checksum mismatch"));
        }
        Ok(response)
    }

    fn acknowledge_packet(&mut self) -> Result<(), QemuError> {
        self.stream.write_all(b"+")?;
        self.stream.flush()?;
        Ok(())
    }

    fn initialize(&mut self) -> Result<(), QemuError> {
        // QEMU's TCP stub waits for the first RSP query rather than sending a
        // packet immediately after accept. The '?' query is the portable GDB
        // RSP synchronization point and returns the current stop reason.
        let response = self.transact(b"?")?;
        stop_reason(&response)?;
        self.refresh_context()
    }

    fn read_byte(&mut self) -> Result<u8, QemuError> {
        let mut byte = [0u8; 1];
        self.stream.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    fn refresh_context(&mut self) -> Result<(), QemuError> {
        let packet = self.transact(b"g")?;
        self.context = match Self::context_from_g_packet(&packet, self.layout) {
            Ok(context) => context,
            Err(_) if packet.len() == Riscv64RegisterLayout::QEMU_INTEGER.packet_length() => {
                self.layout = Riscv64RegisterLayout::QEMU_INTEGER;
                self.capabilities = TargetCapabilities::RV64_INTEGER_BARE_METAL_V1;
                Self::context_from_g_packet(&packet, self.layout)?
            }
            Err(error) => return Err(error),
        };
        Ok(())
    }
}

impl GdbRemote<TcpStream> {
    pub fn connect(address: impl ToSocketAddrs) -> Result<Self, QemuError> {
        let mut backend = Self::new(TcpStream::connect(address)?);
        backend.initialize()?;
        Ok(backend)
    }
}

impl<S: Read + Write> TargetBackend for GdbRemote<S> {
    type Error = QemuError;

    fn capabilities(&self) -> TargetCapabilities {
        self.capabilities
    }

    fn context(&self) -> TargetContext {
        self.context
    }

    fn read_memory(&mut self, address: u64, destination: &mut [u8]) -> Result<(), Self::Error> {
        if destination.is_empty() {
            return Ok(());
        }
        let payload = format!("m{address:x},{:x}", destination.len());
        let response = self.transact(payload.as_bytes())?;
        if response.first() == Some(&b'E') {
            return Err(QemuError::Message(format!(
                "target rejected memory read: {}",
                String::from_utf8_lossy(&response)
            )));
        }
        let bytes = decode_hex(&response)?;
        if bytes.len() != destination.len() {
            return Err(QemuError::Protocol("memory read returned wrong length"));
        }
        destination.copy_from_slice(&bytes);
        Ok(())
    }

    fn write_memory(&mut self, address: u64, source: &[u8]) -> Result<(), Self::Error> {
        let mut payload = format!("M{address:x},{:x}:", source.len()).into_bytes();
        for byte in source {
            payload.extend_from_slice(format!("{byte:02x}").as_bytes());
        }
        let response = self.transact(&payload)?;
        if response != b"OK" {
            return Err(QemuError::Message(format!(
                "target rejected memory write: {}",
                String::from_utf8_lossy(&response)
            )));
        }
        Ok(())
    }

    fn step(&mut self) -> Result<ExecutionOutcome, Self::Error> {
        let response = self.transact(b"s")?;
        let reason = stop_reason(&response)?;
        self.refresh_context()?;
        self.instruction_count = self.instruction_count.saturating_add(1);
        Ok(ExecutionOutcome::Stopped(StopEvent {
            reason,
            pc: self.context.pc,
            instruction_count: self.instruction_count,
        }))
    }

    fn run(&mut self, max_steps: u64) -> Result<ExecutionOutcome, Self::Error> {
        if max_steps == 0 {
            return Ok(ExecutionOutcome::BudgetExhausted {
                pc: self.context.pc,
                instruction_count: self.instruction_count,
            });
        }
        self.step()
    }
}

fn stop_reason(response: &[u8]) -> Result<StopReason, QemuError> {
    let signal = match response.first().copied() {
        Some(b'S') | Some(b'T') if response.len() >= 3 => {
            (hex_digit(response[1])? << 4) | hex_digit(response[2])?
        }
        Some(b'W') | Some(b'X') => return Ok(StopReason::EnvironmentCall),
        _ => return Err(QemuError::Protocol("unsupported stop reply")),
    };
    Ok(match signal {
        4 => StopReason::IllegalInstruction,
        5 => StopReason::Breakpoint,
        7 | 11 => StopReason::InstructionAccessFault,
        _ => StopReason::UnknownTrap,
    })
}

fn decode_hex(bytes: &[u8]) -> Result<Vec<u8>, QemuError> {
    if bytes.len() % 2 != 0 {
        return Err(QemuError::Protocol("odd-length hexadecimal field"));
    }
    bytes
        .chunks_exact(2)
        .map(|pair| Ok((hex_digit(pair[0])? << 4) | hex_digit(pair[1])?))
        .collect()
}

fn hex_digit(byte: u8) -> Result<u8, QemuError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(QemuError::Protocol("invalid hexadecimal digit")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct MockStream {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl MockStream {
        fn new(responses: &[&[u8]]) -> Self {
            let mut input = Vec::new();
            for response in responses {
                input.push(b'+');
                input.extend(packet(response));
            }
            Self {
                input: Cursor::new(input),
                output: Vec::new(),
            }
        }
    }

    impl Read for MockStream {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.input.read(bytes)
        }
    }

    impl Write for MockStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn packet(payload: &[u8]) -> Vec<u8> {
        let checksum = payload
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        let mut packet = b"$".to_vec();
        packet.extend_from_slice(payload);
        packet.extend_from_slice(format!("#{checksum:02x}").as_bytes());
        packet
    }

    fn registers(pc: u64) -> Vec<u8> {
        let mut bytes = vec![0u8; 65 * 8];
        bytes[64 * 8..65 * 8].copy_from_slice(&pc.to_le_bytes());
        bytes
            .iter()
            .flat_map(|byte| format!("{byte:02x}").into_bytes())
            .collect()
    }

    #[test]
    fn decodes_qemu_rv64_register_packet() {
        let mut packet = registers(0x8000_0010);
        packet[0..16].copy_from_slice(b"7856341200000000");
        let context = GdbRemote::<MockStream>::context_from_g_packet(
            &packet,
            Riscv64RegisterLayout::QEMU_DEFAULT,
        )
        .unwrap();
        assert_eq!(context.x[0], 0x1234_5678);
        assert_eq!(context.pc, 0x8000_0010);
    }

    #[test]
    fn decodes_qemu_integer_register_packet_without_floating_registers() {
        let mut bytes = vec![0u8; 33 * 8];
        bytes[32 * 8..33 * 8].copy_from_slice(&0x1000u64.to_le_bytes());
        let packet = bytes
            .iter()
            .flat_map(|byte| format!("{byte:02x}").into_bytes())
            .collect::<Vec<_>>();
        let context = GdbRemote::<MockStream>::context_from_g_packet(
            &packet,
            Riscv64RegisterLayout::QEMU_INTEGER,
        )
        .unwrap();
        assert_eq!(context.pc, 0x1000);
        assert_eq!(context.f[0], 0xffff_ffff_0000_0000);
    }

    #[test]
    fn reads_memory_with_framed_checksum_packet() {
        let stream = MockStream::new(&[b"aabbccdd"]);
        let mut backend = GdbRemote::new(stream);
        let mut bytes = [0u8; 4];
        backend.read_memory(0x100, &mut bytes).unwrap();
        assert_eq!(bytes, [0xaa, 0xbb, 0xcc, 0xdd]);
        assert!(backend.into_inner().output.starts_with(b"$m100,4#"));
    }

    #[test]
    fn rejects_bad_packet_checksum() {
        let mut stream = MockStream::new(&[]);
        stream.input = Cursor::new(b"+$OK#00".to_vec());
        let mut backend = GdbRemote::new(stream);
        let error = backend.write_memory(0, &[1]).unwrap_err();
        assert!(matches!(
            error,
            QemuError::Protocol("packet checksum mismatch")
        ));
    }

    #[test]
    fn single_step_maps_stop_reply_and_refreshes_context() {
        let regs = registers(0x8000_0020);
        let stream = MockStream::new(&[b"S05", &regs]);
        let mut backend = GdbRemote::new(stream);
        let outcome = backend.step().unwrap();
        assert_eq!(
            outcome,
            ExecutionOutcome::Stopped(StopEvent {
                reason: StopReason::Breakpoint,
                pc: 0x8000_0020,
                instruction_count: 1,
            })
        );
    }

    #[test]
    fn zero_budget_does_not_touch_transport() {
        let stream = MockStream::new(&[]);
        let mut backend = GdbRemote::new(stream);
        assert!(matches!(
            backend.run(0).unwrap(),
            ExecutionOutcome::BudgetExhausted { .. }
        ));
        assert!(backend.into_inner().output.is_empty());
    }

    #[test]
    fn initializes_qemu_with_stop_query_and_register_refresh() {
        let regs = registers(0x8000_0050);
        let stream = MockStream::new(&[b"S05", &regs]);
        let mut backend = GdbRemote::new(stream);
        backend.initialize().unwrap();
        assert_eq!(backend.context().pc, 0x8000_0050);
        assert!(backend.into_inner().output.starts_with(b"$?#3f+"));
    }
}
