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

fn parse_number(value: &str) -> Result<u64, String> {
    if let Some(binary) = value.strip_prefix("0b") {
        u64::from_str_radix(binary, 2).map_err(|_| format!("invalid binary value {value}"))
    } else if let Some(hex) = value.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| format!("invalid hexadecimal value {value}"))
    } else {
        value
            .parse::<u64>()
            .map_err(|_| format!("invalid decimal value {value}"))
    }
}

fn parse_fixed(fields: &[&str]) -> Result<(u32, u32), String> {
    let mut mask = 0u32;
    let mut matched = 0u32;
    for field in fields.iter().copied().filter(|field| field.contains('=')) {
        let (range, value) = field
            .split_once('=')
            .ok_or_else(|| format!("invalid field assignment {field}"))?;
        let (msb, lsb) = match range.split_once("..") {
            Some((hi, lo)) => (
                hi.parse::<u32>()
                    .map_err(|_| format!("invalid field high bit {hi}"))?,
                lo.parse::<u32>()
                    .map_err(|_| format!("invalid field low bit {lo}"))?,
            ),
            None => {
                let bit = range
                    .parse::<u32>()
                    .map_err(|_| format!("invalid field bit {range}"))?;
                (bit, bit)
            }
        };
        if msb > 31 || lsb > msb {
            return Err(format!("field range {range} is outside 0..31 or reversed"));
        }
        let width = msb - lsb + 1;
        let value = parse_number(value)?;
        let value_limit = 1u64 << width;
        if value >= value_limit {
            return Err(format!("value {value} does not fit field {range}"));
        }
        let field_mask = if width == 32 {
            u32::MAX
        } else {
            ((1u32 << width) - 1) << lsb
        };
        if mask & field_mask != 0 {
            return Err(format!("overlapping fixed fields at {range}"));
        }
        mask |= field_mask;
        matched |= ((value as u32) << lsb) & field_mask;
    }
    Ok((mask, matched))
}

fn read_r2_commit(manifest: &str) -> Result<String, String> {
    let mut in_r2 = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line == "[r2]" {
            in_r2 = true;
            continue;
        }
        if line.starts_with('[') {
            in_r2 = false;
        }
        if in_r2 && line.starts_with("commit = ") {
            let commit = line
                .strip_prefix("commit = ")
                .and_then(|value| value.strip_prefix('"'))
                .and_then(|value| value.strip_suffix('"'))
                .ok_or_else(|| "malformed R2 commit entry".to_string())?;
            if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(format!("R2 commit is not a full hexadecimal SHA: {commit}"));
            }
            return Ok(commit.to_string());
        }
    }
    Err("R2 commit is missing from norms/manifest.toml".to_string())
}

fn encodings_overlap(left_mask: u32, left_match: u32, right_mask: u32, right_match: u32) -> bool {
    (left_match ^ right_match) & (left_mask & right_mask) == 0
}

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=../../norms/manifest.toml");
    println!("cargo:rerun-if-changed=../../norms/r2/SHA256SUMS");
    let manifest = fs::read_to_string("../../norms/manifest.toml")
        .expect("normative manifest must be available");
    let r2_commit = read_r2_commit(&manifest).unwrap_or_else(|error| panic!("{error}"));
    let mut generated = String::from("pub static GENERATED_OPCODES: &[Opcode] = &[\n");
    let mut addi = None;
    let mut encodings: Vec<(String, String, u32, u32)> = Vec::new();
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
            let (mask, matched) = parse_fixed(&tokens[1..])
                .unwrap_or_else(|error| panic!("invalid R2 source {source_name}:{line}: {error}"));
            if let Some((other_mnemonic, other_source, _other_mask, _other_match)) =
                encodings.iter().find(|(_, _, other_mask, other_match)| {
                    encodings_overlap(mask, matched, *other_mask, *other_match)
                })
            {
                // R2 operand constraints such as rd_n0 are not part of mask/match.
                // Keep reporting these semantic subspaces, but reject exact duplicates.
                if mask == *_other_mask && matched == *_other_match {
                    panic!(
                        "duplicate R2 encoding: {source_name}:{mnemonic} and {other_source}:{other_mnemonic}"
                    );
                }
                println!(
                    "cargo:warning=overlapping R2 encodings: {source_name}:{mnemonic} and {other_source}:{other_mnemonic}"
                );
            }
            encodings.push((
                mnemonic.to_string(),
                (*source_name).to_string(),
                mask,
                matched,
            ));
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
        "pub const ADDI_MASK: u32 = 0x{addi_mask:08x};\npub const ADDI_MATCH: u32 = 0x{addi_match:08x};\npub const R2_COMMIT: &str = \"{r2_commit}\";\n"
    ));
    fs::write(out_dir.join("opcode.rs"), generated).unwrap();
}
