use std::{env, fs, path::PathBuf};

const SOURCES: &[&str] = &[
    "rv_i",
    "rv64_i",
    "rv_m",
    "rv64_m",
    "rv_f",
    "rv64_f",
    "rv_d",
    "rv64_d",
    "rv_zicsr",
    "rv_zifencei",
    "rv_c",
    "rv64_c",
];

fn parse_fixed(fields: &[&str]) -> (u32, u32) {
    let mut mask = 0u32;
    let mut matched = 0u32;
    for field in fields.iter().copied().filter(|field| field.contains('=')) {
        let (range, value) = field.split_once('=').expect("R2 field assignment");
        let (msb, lsb) = match range.split_once("..") {
            Some((hi, lo)) => (hi.parse::<u32>().unwrap(), lo.parse::<u32>().unwrap()),
            None => {
                let bit = range.parse::<u32>().unwrap();
                (bit, bit)
            }
        };
        let width = msb - lsb + 1;
        let value = if let Some(binary) = value.strip_prefix("0b") {
            u32::from_str_radix(binary, 2).unwrap()
        } else if let Some(hex) = value.strip_prefix("0x") {
            u32::from_str_radix(hex, 16).unwrap()
        } else if let Ok(decimal) = value.parse::<u32>() {
            decimal
        } else {
            // R2 also uses assignments such as rs2=rs1 to express an
            // operand constraint; it is not a constant mask bit.
            continue;
        };
        let field_mask = ((1u32 << width) - 1) << lsb;
        mask |= field_mask;
        matched |= (value << lsb) & field_mask;
    }
    (mask, matched)
}

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut generated = String::from("pub static GENERATED_OPCODES: &[Opcode] = &[\n");
    let mut addi = None;
    for source_name in SOURCES {
        let path = format!("../../norms/r2/extensions/{source_name}");
        println!("cargo:rerun-if-changed={path}");
        let source = fs::read_to_string(&path).expect("pinned R2 extension source");
        for line in source.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('$') {
                continue;
            }
            let tokens: Vec<_> = line.split_whitespace().collect();
            if tokens.len() < 2 {
                continue;
            }
            let mnemonic = tokens[0];
            let (mask, matched) = parse_fixed(&tokens[1..]);
            let fields: Vec<_> = tokens[1..]
                .iter()
                .copied()
                .filter(|token| !token.contains('='))
                .collect();
            generated.push_str(&format!(
                "    Opcode {{ mnemonic: \"{mnemonic}\", extension: \"{source_name}\", mask: 0x{mask:08x}, match_value: 0x{matched:08x}, fields: &["
            ));
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    generated.push_str(", ");
                }
                generated.push_str(&format!("\"{field}\""));
            }
            generated.push_str("] },\n");
            if mnemonic == "addi" && addi.is_none() {
                addi = Some((mask, matched));
            }
        }
    }
    generated.push_str("];\n");
    let (addi_mask, addi_match) = addi.expect("R2 profile sources must contain addi");
    generated.push_str(&format!(
        "pub const ADDI_MASK: u32 = 0x{addi_mask:08x};\npub const ADDI_MATCH: u32 = 0x{addi_match:08x};\n"
    ));
    fs::write(out_dir.join("opcode.rs"), generated).unwrap();
}
