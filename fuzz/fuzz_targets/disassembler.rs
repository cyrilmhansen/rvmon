#![no_main]

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use luna_disassembler::disassemble_word;

fuzz_target!(|data: &[u8]| {
    let symbols = BTreeMap::new();
    for (index, chunk) in data.chunks_exact(4).enumerate() {
        let word = u32::from_le_bytes(chunk.try_into().expect("chunk is four bytes"));
        let _ = disassemble_word((index as u64).wrapping_mul(4), word, &symbols);
    }
});
