#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memory {
    bytes: Vec<u8>,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self {
            bytes: vec![0; size],
        }
    }
    fn range(&self, address: u64, width: usize) -> Result<std::ops::Range<usize>> {
        let start = usize::try_from(address)
            .map_err(|_| Diagnostic::error("MEM-ADDRESS-001", "address does not fit host index"))?;
        let end = start
            .checked_add(width)
            .ok_or_else(|| Diagnostic::error("MEM-ADDRESS-002", "address range overflow"))?;
        if end > self.bytes.len() {
            return Err(Diagnostic::error(
                "MEM-ACCESS-001",
                "target memory access is unmapped",
            ));
        }
        Ok(start..end)
    }
    pub fn load32(&self, address: u64) -> Result<u32> {
        let range = self.range(address, 4)?;
        Ok(u32::from_le_bytes(self.bytes[range].try_into().unwrap()))
    }
    pub fn store8(&mut self, address: u64, value: u8) -> Result<()> {
        let range = self.range(address, 1)?;
        self.bytes[range].copy_from_slice(&[value]);
        Ok(())
    }
    pub fn store32(&mut self, address: u64, value: u32) -> Result<()> {
        let range = self.range(address, 4)?;
        self.bytes[range].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}
