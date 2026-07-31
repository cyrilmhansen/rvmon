#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memory {
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    writes: Vec<(u64, u8)>,
}

impl Memory {
    pub fn new(size: usize) -> Self {
        Self {
            bytes: vec![0; size],
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
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
    pub fn load8(&self, address: u64) -> Result<u8> {
        let range = self.range(address, 1)?;
        Ok(self.bytes[range][0])
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

    pub fn transaction(&self) -> Transaction {
        Transaction { writes: Vec::new() }
    }

    pub fn commit(&mut self, transaction: Transaction) -> Result<()> {
        for (address, _) in &transaction.writes {
            self.range(*address, 1)?;
        }
        for (address, value) in transaction.writes {
            self.store8(address, value)?;
        }
        Ok(())
    }
}

impl Transaction {
    pub fn write8(&mut self, address: u64, value: u8) {
        self.writes.push((address, value));
    }
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_commit_is_atomic() {
        let mut memory = Memory::new(4);
        let mut tx = memory.transaction();
        tx.write8(0, 0xaa);
        tx.write8(4, 0xbb);
        assert!(memory.commit(tx).is_err());
        assert_eq!(memory.load32(0).unwrap(), 0);
    }

    #[test]
    fn successful_commit_writes_all_bytes() {
        let mut memory = Memory::new(4);
        let mut tx = memory.transaction();
        tx.write8(0, 0x93);
        tx.write8(1, 0x00);
        tx.write8(2, 0x10);
        tx.write8(3, 0x00);
        memory.commit(tx).unwrap();
        assert_eq!(memory.load32(0).unwrap(), 0x0010_0093);
    }
}
