#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use luna_diag::{Diagnostic, Result};
use luna_isa::{
    Addi, Branch, FRegisterRType, Jal, Jalr, Load, Lui, RType, Store, encode_addi, encode_branch,
    encode_f_r, encode_jal, encode_jalr, encode_load, encode_r, encode_store, encode_u,
};

mod expr;
mod parser;
pub use parser::{Operand, OperandKind, ParsedLine, parse_line};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectImage {
    pub text: Vec<u8>,
    pub entry: u64,
    pub symbols: BTreeMap<String, u64>,
    pub constants: BTreeMap<String, i128>,
}

type SymbolValues = BTreeMap<String, i128>;

fn register(name: &str) -> Result<u8> {
    if let Some(number) = name.strip_prefix('x') {
        return number
            .parse::<u8>()
            .ok()
            .filter(|n| *n < 32)
            .ok_or_else(|| Diagnostic::error("ASM-REGISTER-001", "invalid integer register"));
    }
    match name {
        "zero" => Ok(0),
        "ra" => Ok(1),
        "sp" => Ok(2),
        "gp" => Ok(3),
        "tp" => Ok(4),
        "t0" => Ok(5),
        "t1" => Ok(6),
        "t2" => Ok(7),
        "s0" | "fp" => Ok(8),
        "s1" => Ok(9),
        "a0" => Ok(10),
        "a1" => Ok(11),
        "a2" => Ok(12),
        "a3" => Ok(13),
        "a4" => Ok(14),
        "a5" => Ok(15),
        "a6" => Ok(16),
        "a7" => Ok(17),
        "s2" => Ok(18),
        "s3" => Ok(19),
        "s4" => Ok(20),
        "s5" => Ok(21),
        "s6" => Ok(22),
        "s7" => Ok(23),
        "s8" => Ok(24),
        "s9" => Ok(25),
        "s10" => Ok(26),
        "s11" => Ok(27),
        "t3" => Ok(28),
        "t4" => Ok(29),
        "t5" => Ok(30),
        "t6" => Ok(31),
        _ => Err(Diagnostic::error("ASM-REGISTER-001", "unknown register")),
    }
}

fn floating_register(name: &str) -> Result<u8> {
    name.strip_prefix('f')
        .and_then(|number| number.parse::<u8>().ok())
        .filter(|number| *number < 32)
        .ok_or_else(|| Diagnostic::error("ASM-FREGISTER-001", "invalid floating register"))
}

fn operand_text(operand: &Operand) -> Result<&str> {
    match &operand.kind {
        OperandKind::Register(value)
        | OperandKind::Integer(value)
        | OperandKind::Symbol(value)
        | OperandKind::Expression(value) => Ok(value),
        _ => Err(
            Diagnostic::error("ASM-OPERAND-002", "expected a scalar operand")
                .at(operand.span.line, operand.span.column),
        ),
    }
}

fn memory_operand(operand: &Operand, symbols: &SymbolValues) -> Result<(i16, u8)> {
    let OperandKind::Memory { offset, base } = &operand.kind else {
        return Err(
            Diagnostic::error("ASM-MEMORY-001", "memory operand must be imm(register)")
                .at(operand.span.line, operand.span.column),
        );
    };
    let immediate = expr::evaluate(offset, symbols)
        .and_then(|value| {
            i16::try_from(value).map_err(|_| {
                Diagnostic::error("ASM-IMMEDIATE-001", "memory immediate out of range")
            })
        })
        .map_err(|error| error.at(operand.span.line, operand.span.column))?;
    Ok((immediate, register(base)?))
}

pub fn assemble(source: &str) -> Result<ObjectImage> {
    let parsed = parse_line(source)?;
    if !parsed.labels.is_empty() {
        return Err(Diagnostic::error(
            "ASM-LABEL-003",
            "labels are only valid when assembling a program",
        ));
    }
    if matches!(parsed.mnemonic.as_deref(), Some(".equ" | ".set")) {
        return Err(Diagnostic::error(
            "ASM-DIRECTIVE-005",
            ".equ and .set require program assembly",
        ));
    }
    assemble_parsed(&parsed, 0, &BTreeMap::new())
}

fn assemble_parsed(parsed: &ParsedLine, pc: u64, symbols: &SymbolValues) -> Result<ObjectImage> {
    let mnemonic = parsed.mnemonic.as_deref().unwrap_or("");
    let parts = &parsed.operands;
    if mnemonic.starts_with('.') {
        return Ok(ObjectImage {
            text: assemble_directive(parsed, pc, symbols)?,
            entry: 0,
            symbols: BTreeMap::new(),
            constants: BTreeMap::new(),
        });
    }
    let word = match mnemonic {
        "addi" if parts.len() == 3 => encode_addi(Addi {
            rd: register(operand_text(&parts[0])?)?,
            rs1: register(operand_text(&parts[1])?)?,
            imm: i16::try_from(immediate(
                &parts[2],
                symbols,
                i16::MIN as i128,
                i16::MAX as i128,
            )?)
            .unwrap(),
        })?,
        "add" | "sub" if parts.len() == 3 => encode_r(
            mnemonic,
            RType {
                rd: register(operand_text(&parts[0])?)?,
                rs1: register(operand_text(&parts[1])?)?,
                rs2: register(operand_text(&parts[2])?)?,
            },
        )?,
        "lui" | "auipc" if parts.len() == 2 => encode_u_instruction(
            mnemonic,
            Lui {
                rd: register(operand_text(&parts[0])?)?,
                imm20: u32::try_from(immediate(&parts[1], symbols, 0, 0x000f_ffff)?)
                    .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid U immediate"))?,
            },
        )?,
        "lw" | "ld" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(&parts[1], symbols)?;
            encode_load(
                mnemonic,
                Load {
                    rd: register(operand_text(&parts[0])?)?,
                    rs1,
                    imm,
                },
            )?
        }
        "sw" | "sd" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(&parts[1], symbols)?;
            encode_store(
                mnemonic,
                Store {
                    rs2: register(operand_text(&parts[0])?)?,
                    rs1,
                    imm,
                },
            )?
        }
        "beq" | "bne" if parts.len() == 3 => encode_branch(
            mnemonic,
            Branch {
                rs1: register(operand_text(&parts[0])?)?,
                rs2: register(operand_text(&parts[1])?)?,
                imm: i16::try_from(immediate(
                    &parts[2],
                    symbols,
                    i16::MIN as i128,
                    i16::MAX as i128,
                )?)
                .unwrap(),
            },
        )?,
        "jal" if parts.len() == 2 => encode_jal(Jal {
            rd: register(operand_text(&parts[0])?)?,
            imm: i32::try_from(immediate(
                &parts[1],
                symbols,
                i32::MIN as i128,
                i32::MAX as i128,
            )?)
            .map_err(|_| Diagnostic::error("ASM-IMMEDIATE-001", "invalid jump immediate"))?,
        })?,
        "jalr" if parts.len() == 2 => {
            let (imm, rs1) = memory_operand(&parts[1], symbols)?;
            encode_jalr(Jalr {
                rd: register(operand_text(&parts[0])?)?,
                rs1,
                imm,
            })?
        }
        "fadd.s" if parts.len() == 3 => encode_f_r(
            "fadd.s",
            FRegisterRType {
                rd: floating_register(operand_text(&parts[0])?)?,
                rs1: floating_register(operand_text(&parts[1])?)?,
                rs2: floating_register(operand_text(&parts[2])?)?,
                rm: 7,
            },
        )?,
        "fadd.d" if parts.len() == 3 => encode_f_r(
            "fadd.d",
            FRegisterRType {
                rd: floating_register(operand_text(&parts[0])?)?,
                rs1: floating_register(operand_text(&parts[1])?)?,
                rs2: floating_register(operand_text(&parts[2])?)?,
                rm: 7,
            },
        )?,
        "" => {
            return Err(Diagnostic::error("ASM-OPERAND-001", "missing instruction"));
        }
        _ => {
            return Err(Diagnostic::error(
                "ASM-BOOT-UNSUPPORTED",
                "bootstrap assembler accepts integer forms plus fadd.s and fadd.d",
            ));
        }
    };
    Ok(ObjectImage {
        text: word.to_le_bytes().to_vec(),
        entry: 0,
        symbols: BTreeMap::new(),
        constants: BTreeMap::new(),
    })
}

fn encode_u_instruction(mnemonic: &str, instruction: Lui) -> Result<u32> {
    if instruction.rd > 31 || instruction.imm20 > 0x000f_ffff {
        return Err(Diagnostic::error(
            "ASM-IMMEDIATE-001",
            "U-type register or immediate out of range",
        ));
    }
    encode_u(mnemonic, instruction)
}

fn immediate(
    operand: &Operand,
    symbols: &SymbolValues,
    minimum: i128,
    maximum: i128,
) -> Result<i128> {
    let value = expr::evaluate(operand_text(operand)?, symbols)
        .map_err(|error| error.at(operand.span.line, operand.span.column))?;
    if !(minimum..=maximum).contains(&value) {
        return Err(
            Diagnostic::error("ASM-IMMEDIATE-001", "immediate out of range")
                .at(operand.span.line, operand.span.column),
        );
    }
    Ok(value)
}

fn assemble_directive(parsed: &ParsedLine, pc: u64, symbols: &SymbolValues) -> Result<Vec<u8>> {
    match parsed.mnemonic.as_deref().unwrap_or_default() {
        ".equ" | ".set" => Ok(Vec::new()),
        ".byte" => data_values(&parsed.operands, 1, symbols),
        ".half" => data_values(&parsed.operands, 2, symbols),
        ".word" => data_values(&parsed.operands, 4, symbols),
        ".dword" => data_values(&parsed.operands, 8, symbols),
        ".ascii" | ".asciz" | ".string" => {
            let mut bytes = Vec::new();
            for operand in &parsed.operands {
                let OperandKind::String(value) = &operand.kind else {
                    return Err(
                        Diagnostic::error("ASM-DATA-001", "expected a string literal")
                            .at(operand.span.line, operand.span.column),
                    );
                };
                bytes.extend_from_slice(value.as_bytes());
                if matches!(parsed.mnemonic.as_deref(), Some(".asciz" | ".string")) {
                    bytes.push(0);
                }
            }
            Ok(bytes)
        }
        ".align" => {
            if parsed.operands.len() != 1 {
                return Err(Diagnostic::error(
                    "ASM-DIRECTIVE-001",
                    ".align expects one operand",
                ));
            }
            let exponent = immediate(&parsed.operands[0], symbols, 0, 63)?;
            let boundary = 1u64 << u32::try_from(exponent).unwrap();
            alignment_padding(pc, boundary)
        }
        ".balign" => {
            if parsed.operands.len() != 1 {
                return Err(Diagnostic::error(
                    "ASM-DIRECTIVE-001",
                    ".balign expects one operand",
                ));
            }
            let boundary = u64::try_from(immediate(
                &parsed.operands[0],
                symbols,
                1,
                i128::from(u64::MAX),
            )?)
            .map_err(|_| Diagnostic::error("ASM-DIRECTIVE-002", "alignment is too large"))?;
            alignment_padding(pc, boundary)
        }
        _ => Err(Diagnostic::error(
            "ASM-DIRECTIVE-003",
            "unsupported directive",
        )),
    }
}

fn alignment_padding(pc: u64, boundary: u64) -> Result<Vec<u8>> {
    if boundary == 0 {
        return Err(Diagnostic::error(
            "ASM-DIRECTIVE-002",
            "alignment must be greater than zero",
        ));
    }
    let padding = (boundary - (pc % boundary)) % boundary;
    Ok(vec![
        0;
        usize::try_from(padding).map_err(|_| {
            Diagnostic::error("ASM-DIRECTIVE-002", "alignment is too large")
        })?
    ])
}

fn data_values(operands: &[Operand], width: usize, symbols: &SymbolValues) -> Result<Vec<u8>> {
    let bits = width * 8;
    let minimum = -(1i128 << (bits - 1));
    let maximum = (1i128 << bits) - 1;
    let mut bytes = Vec::with_capacity(operands.len() * width);
    for operand in operands {
        let value = immediate(operand, symbols, minimum, maximum)? as u128;
        let raw = value.to_le_bytes();
        bytes.extend_from_slice(&raw[..width]);
    }
    Ok(bytes)
}

pub fn assemble_program(source: &str) -> Result<ObjectImage> {
    let lines: Vec<_> = source.lines().map(parse_line).collect::<Result<_>>()?;
    let mut symbols = BTreeMap::new();
    let mut values = SymbolValues::new();
    let mut constants = BTreeMap::new();
    let mut equ_values = SymbolValues::new();
    let mut equ_names = BTreeSet::new();
    let mut current_global = None;
    let mut pc = 0u64;
    for line in &lines {
        define_labels(
            &line.labels,
            pc,
            &mut symbols,
            &mut values,
            &mut current_global,
        )?;
        define_absolute_directive(
            line,
            current_global.as_deref(),
            &symbols,
            &mut values,
            &mut constants,
            &mut equ_values,
            &mut equ_names,
        )?;
        if line.mnemonic.is_some() {
            let scoped = scoped_symbols(&values, current_global.as_deref());
            let size = line_size(line, pc, &scoped)?;
            pc = pc
                .checked_add(size)
                .ok_or_else(|| Diagnostic::error("ASM-ADDRESS-001", "program is too large"))?;
        }
    }

    let mut text = Vec::new();
    let mut emit_values: SymbolValues = symbols
        .iter()
        .map(|(name, address)| (name.clone(), i128::from(*address)))
        .collect();
    emit_values.extend(equ_values);
    let mut current_global = None;
    pc = 0;
    for line in lines {
        update_scope(&line.labels, &mut current_global)?;
        if line.mnemonic.is_none() {
            continue;
        }
        if line.mnemonic.as_deref() == Some(".set") {
            apply_set_directive(
                &line,
                current_global.as_deref(),
                &symbols,
                &equ_names,
                &mut emit_values,
            )?;
            continue;
        }
        if line.mnemonic.as_deref() == Some(".equ") {
            continue;
        }
        let scoped = scoped_symbols(&emit_values, current_global.as_deref());
        let resolved = resolve_control_label(line, pc, &scoped)?;
        let image = assemble_parsed(&resolved, pc, &scoped)?;
        text.extend_from_slice(&image.text);
        pc += image.text.len() as u64;
    }
    let entry = symbols.get("_start").copied().unwrap_or(0);
    Ok(ObjectImage {
        text,
        entry,
        symbols,
        constants,
    })
}

fn is_local_label(label: &str) -> bool {
    label.starts_with(".L")
}

fn local_symbol_key(scope: &str, label: &str) -> String {
    format!("{scope}::{label}")
}

fn define_labels(
    labels: &[String],
    pc: u64,
    symbols: &mut BTreeMap<String, u64>,
    values: &mut SymbolValues,
    current_global: &mut Option<String>,
) -> Result<()> {
    for label in labels {
        let key = if is_local_label(label) {
            let Some(scope) = current_global.as_deref() else {
                return Err(Diagnostic::error(
                    "ASM-LABEL-001",
                    "local label requires a preceding global label",
                ));
            };
            local_symbol_key(scope, label)
        } else {
            current_global.replace(label.clone());
            label.clone()
        };
        if values.contains_key(&key) {
            return Err(Diagnostic::error("ASM-LABEL-002", "duplicate label"));
        }
        symbols.insert(key.clone(), pc);
        values.insert(key, i128::from(pc));
    }
    Ok(())
}

fn update_scope(labels: &[String], current_global: &mut Option<String>) -> Result<()> {
    for label in labels {
        if is_local_label(label) {
            if current_global.is_none() {
                return Err(Diagnostic::error(
                    "ASM-LABEL-001",
                    "local label requires a preceding global label",
                ));
            }
        } else {
            current_global.replace(label.clone());
        }
    }
    Ok(())
}

fn scoped_symbols(symbols: &SymbolValues, current_global: Option<&str>) -> SymbolValues {
    let mut scoped = symbols.clone();
    let Some(global) = current_global else {
        return scoped;
    };
    let prefix = format!("{global}::");
    for (key, value) in symbols {
        if let Some(local) = key.strip_prefix(&prefix) {
            scoped.insert(local.to_owned(), *value);
        }
    }
    scoped
}

fn definition_parts(line: &ParsedLine) -> Result<(&str, &str)> {
    if line.operands.len() != 2 {
        return Err(Diagnostic::error(
            "ASM-DIRECTIVE-004",
            ".equ and .set expect a name and one expression",
        ));
    }
    let name = match &line.operands[0].kind {
        OperandKind::Symbol(value) => value.as_str(),
        _ => {
            return Err(Diagnostic::error(
                "ASM-SYMBOL-002",
                "symbol definition requires a symbol name",
            )
            .at(line.operands[0].span.line, line.operands[0].span.column));
        }
    };
    let expression = operand_text(&line.operands[1])?;
    Ok((name, expression))
}

fn canonical_definition_name(name: &str, current_global: Option<&str>) -> Result<String> {
    if !is_local_label(name) {
        return Ok(name.to_owned());
    }
    let Some(scope) = current_global else {
        return Err(Diagnostic::error(
            "ASM-LABEL-001",
            "local symbol requires a preceding global label",
        ));
    };
    Ok(local_symbol_key(scope, name))
}

fn evaluate_definition(
    line: &ParsedLine,
    expression: &str,
    values: &SymbolValues,
    current_global: Option<&str>,
) -> Result<i128> {
    let scoped = scoped_symbols(values, current_global);
    expr::evaluate(expression, &scoped)
        .map_err(|error| error.at(line.operands[1].span.line, line.operands[1].span.column))
}

fn define_absolute_directive(
    line: &ParsedLine,
    current_global: Option<&str>,
    labels: &BTreeMap<String, u64>,
    values: &mut SymbolValues,
    constants: &mut BTreeMap<String, i128>,
    equ_values: &mut SymbolValues,
    equ_names: &mut BTreeSet<String>,
) -> Result<()> {
    let Some(mnemonic) = line.mnemonic.as_deref() else {
        return Ok(());
    };
    if !matches!(mnemonic, ".equ" | ".set") {
        return Ok(());
    }
    let (name, expression) = definition_parts(line)?;
    let key = canonical_definition_name(name, current_global)?;
    if labels.contains_key(&key) {
        return Err(Diagnostic::error(
            "ASM-SYMBOL-004",
            "cannot redefine a label as an absolute symbol",
        ));
    }
    if mnemonic == ".equ" {
        if values.contains_key(&key) {
            return Err(Diagnostic::error(
                "ASM-SYMBOL-003",
                ".equ symbols are immutable",
            ));
        }
        let value = evaluate_definition(line, expression, values, current_global)?;
        values.insert(key.clone(), value);
        constants.insert(key.clone(), value);
        equ_values.insert(key.clone(), value);
        equ_names.insert(key);
    } else {
        if equ_names.contains(&key) {
            return Err(Diagnostic::error(
                "ASM-SYMBOL-003",
                ".equ symbols are immutable",
            ));
        }
        let value = evaluate_definition(line, expression, values, current_global)?;
        values.insert(key.clone(), value);
        constants.insert(key, value);
    }
    Ok(())
}

fn apply_set_directive(
    line: &ParsedLine,
    current_global: Option<&str>,
    labels: &BTreeMap<String, u64>,
    equ_names: &BTreeSet<String>,
    values: &mut SymbolValues,
) -> Result<()> {
    let (name, expression) = definition_parts(line)?;
    let key = canonical_definition_name(name, current_global)?;
    if labels.contains_key(&key) || equ_names.contains(&key) {
        return Err(Diagnostic::error(
            "ASM-SYMBOL-003",
            "cannot redefine an immutable symbol",
        ));
    }
    let value = evaluate_definition(line, expression, values, current_global)?;
    values.insert(key, value);
    Ok(())
}

fn line_size(line: &ParsedLine, pc: u64, symbols: &SymbolValues) -> Result<u64> {
    match line.mnemonic.as_deref().unwrap_or_default() {
        ".equ" | ".set" => return Ok(0),
        ".byte" => return Ok(line.operands.len() as u64),
        ".half" => return Ok((line.operands.len() * 2) as u64),
        ".word" => return Ok((line.operands.len() * 4) as u64),
        ".dword" => return Ok((line.operands.len() * 8) as u64),
        ".ascii" | ".asciz" | ".string" => {
            let mut size = 0u64;
            for operand in &line.operands {
                let OperandKind::String(value) = &operand.kind else {
                    return Err(
                        Diagnostic::error("ASM-DATA-001", "expected a string literal")
                            .at(operand.span.line, operand.span.column),
                    );
                };
                size += value.len() as u64;
                if matches!(line.mnemonic.as_deref(), Some(".asciz" | ".string")) {
                    size += 1;
                }
            }
            return Ok(size);
        }
        ".align" => return Ok(assemble_directive(line, pc, symbols)?.len() as u64),
        ".balign" => return Ok(assemble_directive(line, pc, symbols)?.len() as u64),
        _ => {}
    }
    Ok(4)
}

fn resolve_control_label(
    mut line: ParsedLine,
    pc: u64,
    symbols: &SymbolValues,
) -> Result<ParsedLine> {
    let index = match line.mnemonic.as_deref() {
        Some("beq") | Some("bne") => Some(2),
        Some("jal") => Some(1),
        _ => None,
    };
    if let Some(index) = index {
        if let Some(operand) = line.operands.get_mut(index) {
            let expression = operand_text(operand)?;
            if expr::references_symbol(expression) {
                let target = expr::evaluate(expression, symbols)
                    .map_err(|error| error.at(operand.span.line, operand.span.column))?;
                let offset = target - i128::from(pc);
                operand.kind = OperandKind::Integer(offset.to_string());
            }
        }
    }
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assembles_required_first_program() {
        assert_eq!(
            assemble("addi x1,x0,1").unwrap().text,
            [0x93, 0x00, 0x10, 0x00]
        );
    }
    #[test]
    fn supports_abi_aliases() {
        assert!(assemble("addi ra,zero,1").is_ok());
    }

    #[test]
    fn assembles_fadd_s_with_dynamic_rounding_mode() {
        assert_eq!(
            assemble("fadd.s f3,f1,f2").unwrap().text,
            [0xd3, 0xf1, 0x20, 0x00]
        );
    }

    #[test]
    fn assembles_fadd_d_with_dynamic_rounding_mode() {
        assert_eq!(
            assemble("fadd.d f3,f1,f2").unwrap().text,
            [0xd3, 0xf1, 0x20, 0x02]
        );
    }

    #[test]
    fn assembles_generated_integer_forms() {
        assert!(assemble("add x5,x6,x7").is_ok());
        assert!(assemble("sub x5,x6,x7").is_ok());
        assert!(assemble("lui x3,74565").is_ok());
        assert_eq!(
            assemble("auipc x4,4096").unwrap().text,
            [0x17, 0x02, 0x00, 0x01]
        );
        assert!(assemble("lw x3,8(x4)").is_ok());
        assert!(assemble("sw x3,-8(x4)").is_ok());
        assert!(assemble("beq x1,x2,-4").is_ok());
        assert!(assemble("jal ra,2048").is_ok());
        assert!(assemble("jalr ra,0(x4)").is_ok());
        assert_eq!(
            assemble("ld x3,-8(x4)").unwrap().text,
            [0x83, 0x31, 0x82, 0xff]
        );
        assert_eq!(
            assemble("sd x3,8(x4)").unwrap().text,
            [0x23, 0x34, 0x32, 0x00]
        );
    }

    #[test]
    fn assembles_program_with_labels_and_control_flow() {
        let image = assemble_program("_start: addi x1,x0,1\n       beq x1,x1,done\n       addi x1,x0,99\ndone:  addi x2,x0,7").unwrap();
        assert_eq!(image.symbols["_start"], 0);
        assert_eq!(image.symbols["done"], 12);
        assert_eq!(image.text.len(), 16);
    }

    #[test]
    fn assembles_data_directives_and_alignment() {
        let image =
            assemble_program(".byte 1, 2+3\n.half 0x1234\n.byte 9\n.align 2\n.asciz \"ok\"")
                .unwrap();
        assert_eq!(image.text, [1, 5, 0x34, 0x12, 9, 0, 0, 0, b'o', b'k', 0]);
    }

    #[test]
    fn assembles_string_and_byte_alignment_directives() {
        let image = assemble_program(".byte 1\n.balign 4\n.string \"ok\"").unwrap();
        assert_eq!(image.text, [1, 0, 0, 0, b'o', b'k', 0]);

        let image = assemble_program(".byte 1\n.balign 3\n.ascii \"ok\"").unwrap();
        assert_eq!(image.text, [1, 0, 0, b'o', b'k']);
    }

    #[test]
    fn rejects_invalid_byte_alignment_directives() {
        let error = assemble_program(".balign 0").unwrap_err();
        assert_eq!(error.code, "ASM-IMMEDIATE-001");
        let error = assemble_program(".balign 4, 0").unwrap_err();
        assert_eq!(error.code, "ASM-DIRECTIVE-001");
    }

    #[test]
    fn resolves_forward_data_symbol_and_expression_immediate() {
        let image = assemble_program(".word target + 4\naddi x1,x0,1+2\ntarget: .byte 7").unwrap();
        assert_eq!(&image.text[..4], &[0x0c, 0, 0, 0]);
        assert_eq!(&image.text[4..8], &[0x93, 0x00, 0x30, 0x00]);
        assert_eq!(image.symbols["target"], 8);
    }

    #[test]
    fn preserves_numeric_control_offsets_after_origin() {
        let image = assemble_program("addi x1,x0,1\nbeq x1,x1,4").unwrap();
        assert_eq!(&image.text[4..8], &[0x63, 0x82, 0x10, 0x00]);
    }

    #[test]
    fn resolves_local_labels_per_global_scope() {
        let image = assemble_program(
            "_start: jal ra,.Ldone\n        .word .Ldone\n.Ldone: addi x1,x0,1\nnext:   jal ra,.Ldone\n.Ldone: addi x2,x0,2",
        )
        .unwrap();
        assert_eq!(image.symbols["_start::.Ldone"], 8);
        assert_eq!(image.symbols["next::.Ldone"], 16);
        assert_eq!(image.text.len(), 20);
    }

    #[test]
    fn rejects_local_label_without_global_scope() {
        let error = assemble_program(".Lloop: addi x1,x0,1").unwrap_err();
        assert_eq!(error.code, "ASM-LABEL-001");
    }

    #[test]
    fn rejects_duplicate_global_labels() {
        let error = assemble_program("value: .byte 1\nvalue: .byte 2").unwrap_err();
        assert_eq!(error.code, "ASM-LABEL-002");
    }

    #[test]
    fn resolves_equ_and_set_in_data_and_instructions() {
        let image =
            assemble_program(".equ BASE, 3\n.set VALUE, BASE + 4\n.word VALUE\naddi x1,x0,VALUE")
                .unwrap();
        assert_eq!(&image.text[..4], &[7, 0, 0, 0]);
        assert_eq!(&image.text[4..8], &[0x93, 0x00, 0x70, 0x00]);
        assert_eq!(image.constants["BASE"], 3);
        assert_eq!(image.constants["VALUE"], 7);
    }

    #[test]
    fn preserves_signed_absolute_constants_for_data() {
        let image = assemble_program(".equ NEGATIVE, -1\n.byte NEGATIVE\n.word NEGATIVE").unwrap();
        assert_eq!(image.text, [0xff, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(image.constants["NEGATIVE"], -1);
    }

    #[test]
    fn set_is_sequential_and_equ_is_immutable() {
        let image =
            assemble_program(".set value, 1\n.word value\n.set value, 2\n.word value").unwrap();
        assert_eq!(image.text, [1, 0, 0, 0, 2, 0, 0, 0]);

        let error = assemble_program(".equ value, 1\n.set value, 2").unwrap_err();
        assert_eq!(error.code, "ASM-SYMBOL-003");
    }

    #[test]
    fn rejects_absolute_directives_in_single_line_mode() {
        let error = assemble(".equ value, 1").unwrap_err();
        assert_eq!(error.code, "ASM-DIRECTIVE-005");
    }

    #[test]
    fn rejects_data_overflow() {
        let error = assemble(".byte 256").unwrap_err();
        assert_eq!(error.code, "ASM-IMMEDIATE-001");
    }
}
