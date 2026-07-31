use std::{env, fmt::Write, fs, path::PathBuf};

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

fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut message = input.to_vec();
    let bit_length = (message.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let mut working = state;
        for index in 0..64 {
            let s1 = working[4].rotate_right(6)
                ^ working[4].rotate_right(11)
                ^ working[4].rotate_right(25);
            let choice = (working[4] & working[5]) ^ ((!working[4]) & working[6]);
            let temp1 = working[7]
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = working[0].rotate_right(2)
                ^ working[0].rotate_right(13)
                ^ working[0].rotate_right(22);
            let majority =
                (working[0] & working[1]) ^ (working[0] & working[2]) ^ (working[1] & working[2]);
            let temp2 = s0.wrapping_add(majority);
            working[7] = working[6];
            working[6] = working[5];
            working[5] = working[4];
            working[4] = working[3].wrapping_add(temp1);
            working[3] = working[2];
            working[2] = working[1];
            working[1] = working[0];
            working[0] = temp1.wrapping_add(temp2);
        }
        for index in 0..8 {
            state[index] = state[index].wrapping_add(working[index]);
        }
    }
    let mut result = String::with_capacity(64);
    for word in state {
        write!(&mut result, "{word:08x}").expect("writing a String cannot fail");
    }
    result
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
    let mut extension_counts = Vec::new();
    for source_name in SOURCES {
        let path = format!("../../norms/r2/extensions/{source_name}");
        println!("cargo:rerun-if-changed={path}");
        let source = fs::read_to_string(&path).expect("pinned R2 extension source");
        let instruction_bits = if *source_name == "rv_c" || *source_name == "rv64_c" {
            16
        } else {
            32
        };
        let mut instruction_count = 0;
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
            instruction_count += 1;
            let fields: Vec<_> = tokens[1..]
                .iter()
                .copied()
                .filter(|token| !token.contains('='))
                .collect();
            generated.push_str(&format!(
                "    Opcode {{ mnemonic: \"{mnemonic}\", extension: \"{source_name}\", instruction_bits: {instruction_bits}, mask: 0x{mask:08x}, match_value: 0x{matched:08x}, fields: &["
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
        extension_counts.push((*source_name, instruction_bits, instruction_count));
    }
    generated.push_str("];\n");
    let table_hash = sha256_hex(generated.as_bytes());
    let (addi_mask, addi_match) = addi.expect("R2 profile sources must contain addi");
    generated.push_str(&format!(
        "pub const ADDI_MASK: u32 = 0x{addi_mask:08x};\npub const ADDI_MATCH: u32 = 0x{addi_match:08x};\npub const R2_COMMIT: &str = \"{r2_commit}\";\npub const GENERATED_OPCODE_COUNT: usize = {};\npub const R2_OPCODE_TABLE_SHA256: &str = \"{table_hash}\";\npub static GENERATED_EXTENSIONS: &[GeneratedExtension] = &[\n",
        encodings.len()
    ));
    for (name, instruction_bits, instruction_count) in extension_counts {
        generated.push_str(&format!(
            "    GeneratedExtension {{ name: \"{name}\", instruction_bits: {instruction_bits}, instruction_count: {instruction_count} }},\n"
        ));
    }
    generated.push_str("];\n");
    fs::write(out_dir.join("opcode.rs"), generated).unwrap();
}
