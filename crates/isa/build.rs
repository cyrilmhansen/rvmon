use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=../../norms/r2/rv_i");
    let source = fs::read_to_string("../../norms/r2/rv_i").expect("pinned R2 extract");
    let line = source
        .lines()
        .find(|line| line.trim_start().starts_with("addi "))
        .expect("R2 extract must contain addi");
    let mut mask = 0u32;
    let mut matched = 0u32;
    for field in line.split_whitespace().skip(4) {
        let (range, value) = field.split_once('=').expect("R2 field assignment");
        let (msb, lsb) = match range.split_once("..") {
            Some((hi, lo)) => (hi.parse::<u32>().unwrap(), lo.parse::<u32>().unwrap()),
            None => {
                let bit = range.parse::<u32>().unwrap();
                (bit, bit)
            }
        };
        let width = msb - lsb + 1;
        let value = value.strip_prefix("0x").map_or_else(
            || value.parse::<u32>().unwrap(),
            |hex| u32::from_str_radix(hex, 16).unwrap(),
        );
        let field_mask = ((1u32 << width) - 1) << lsb;
        mask |= field_mask;
        matched |= (value << lsb) & field_mask;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("opcode.rs");
    fs::write(out, format!(
        "pub const ADDI_MASK: u32 = 0x{mask:08x};\npub const ADDI_MATCH: u32 = 0x{matched:08x};\n"
    )).unwrap();
}
