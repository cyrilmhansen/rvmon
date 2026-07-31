#![forbid(unsafe_code)]

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use luna_diag::{Diagnostic, Result};
use luna_floatfmt::{FloatFormat, parse_float_literal};
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
    pub listing: Vec<ListingEntry>,
    pub sections: Vec<SectionImage>,
}

/// Controls the optional source-file include loader.
///
/// Includes are disabled when `include_roots` is empty. Every included file
/// must resolve below one of the configured roots after symlink resolution;
/// the loader never falls back to the process working directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyOptions {
    pub include_roots: Vec<PathBuf>,
    pub source_path: Option<PathBuf>,
    pub max_include_depth: usize,
    pub max_include_bytes: usize,
    pub max_include_files: usize,
}

impl Default for AssemblyOptions {
    fn default() -> Self {
        Self {
            include_roots: Vec::new(),
            source_path: None,
            max_include_depth: MAX_INCLUDE_DEPTH,
            max_include_bytes: MAX_INCLUDE_BYTES,
            max_include_files: MAX_INCLUDE_FILES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListingEntry {
    pub source_line: u32,
    pub address: u64,
    pub section: String,
    pub source: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionImage {
    pub name: String,
    pub flags: String,
    pub address: u64,
    pub alignment: u64,
    pub bytes: Vec<u8>,
}

/// Render the source listing in a stable, human-readable text format.
///
/// Each line is `line address section bytes | original source`. Empty byte
/// lists are represented by `-`; the output contains no timestamps or host
/// paths and is therefore suitable for reproducible exports.
pub fn render_listing(image: &ObjectImage) -> String {
    let mut output = String::new();
    for entry in &image.listing {
        let mut bytes = String::new();
        for (index, byte) in entry.bytes.iter().enumerate() {
            if index != 0 {
                bytes.push(' ');
            }
            write!(bytes, "{byte:02x}").expect("writing to String cannot fail");
        }
        if bytes.is_empty() {
            bytes.push('-');
        }
        writeln!(
            output,
            "{:04} 0x{:016x} {:<12} {:<47} | {}",
            entry.source_line, entry.address, entry.section, bytes, entry.source
        )
        .expect("writing to String cannot fail");
    }
    output
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SectionState {
    name: String,
    flags: String,
    address: Option<u64>,
    alignment: u64,
    bytes: Vec<u8>,
}

type SymbolValues = BTreeMap<String, i128>;

const MAX_MACRO_DEFINITIONS: usize = 256;
const MAX_MACRO_BODY_LINES: usize = 4096;
const MAX_MACRO_PARAMETERS: usize = 32;
const MAX_MACRO_DEPTH: usize = 32;
const MAX_EXPANDED_LINES: usize = 65_536;
const MAX_INCLUDE_DEPTH: usize = 32;
const MAX_INCLUDE_BYTES: usize = 8 * 1024 * 1024;
const MAX_INCLUDE_FILES: usize = 4096;
const MAX_CONDITIONAL_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExpandedLine {
    text: String,
    source_line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MacroDefinition {
    parameters: Vec<String>,
    body: Vec<ExpandedLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConditionalDirective {
    If(String),
    Else,
    EndIf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConditionalFrame {
    parent_active: bool,
    condition_true: bool,
    branch_active: bool,
    else_seen: bool,
}

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
    let mut image = assemble_parsed(&parsed, 0, &BTreeMap::new())?;
    image.listing.push(ListingEntry {
        source_line: 1,
        address: 0,
        section: ".text".into(),
        source: source.to_owned(),
        bytes: image.text.clone(),
    });
    Ok(image)
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
            listing: Vec::new(),
            sections: Vec::new(),
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
        listing: Vec::new(),
        sections: Vec::new(),
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
        ".binary16" => float_values(&parsed.operands, FloatFormat::Binary16),
        ".float" => float_values(&parsed.operands, FloatFormat::Binary32),
        ".double" => float_values(&parsed.operands, FloatFormat::Binary64),
        ".binary128" => float_values(&parsed.operands, FloatFormat::Binary128),
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

fn float_values(operands: &[Operand], format: FloatFormat) -> Result<Vec<u8>> {
    let width = match format {
        FloatFormat::Binary16 => 2,
        FloatFormat::Binary32 => 4,
        FloatFormat::Binary64 => 8,
        FloatFormat::Binary128 => 16,
    };
    let mut bytes = Vec::with_capacity(operands.len() * width);
    for operand in operands {
        let literal = match &operand.kind {
            OperandKind::BitPattern {
                width: pattern_width,
                value,
            } => {
                let expected_width = width * 8;
                if *pattern_width != expected_width {
                    return Err(Diagnostic::error(
                        "ASM-FLOAT-002",
                        "bit-pattern width does not match directive",
                    )
                    .at(operand.span.line, operand.span.column));
                }
                value.as_str()
            }
            _ => operand_text(operand)?,
        };
        let bits = parse_float_literal(format, literal).ok_or_else(|| {
            Diagnostic::error(
                "ASM-FLOAT-001",
                "invalid or non-representable floating literal",
            )
            .at(operand.span.line, operand.span.column)
        })?;
        let raw = bits.to_le_bytes();
        bytes.extend_from_slice(&raw[..width]);
    }
    Ok(bytes)
}

fn section_spec(line: &ParsedLine) -> Result<Option<(String, String)>> {
    let Some(mnemonic) = line.mnemonic.as_deref() else {
        return Ok(None);
    };
    let builtin = match mnemonic {
        ".text" => Some((".text", "ax")),
        ".rodata" => Some((".rodata", "a")),
        ".data" => Some((".data", "aw")),
        ".bss" => Some((".bss", "aw")),
        _ => None,
    };
    if let Some((name, flags)) = builtin {
        if !line.operands.is_empty() {
            return Err(Diagnostic::error(
                "ASM-SECTION-001",
                "built-in section directives take no operands",
            ));
        }
        return Ok(Some((name.into(), flags.into())));
    }
    if mnemonic != ".section" {
        return Ok(None);
    }
    if line.operands.len() != 2 {
        return Err(Diagnostic::error(
            "ASM-SECTION-001",
            ".section expects a name and flags string",
        ));
    }
    let name = match &line.operands[0].kind {
        OperandKind::Symbol(value) | OperandKind::String(value) => value.clone(),
        _ => {
            return Err(Diagnostic::error(
                "ASM-SECTION-002",
                "section name must be a symbol or string",
            )
            .at(line.operands[0].span.line, line.operands[0].span.column));
        }
    };
    let OperandKind::String(flags) = &line.operands[1].kind else {
        return Err(
            Diagnostic::error("ASM-SECTION-003", "section flags must be a string")
                .at(line.operands[1].span.line, line.operands[1].span.column),
        );
    };
    if name.is_empty() || flags.is_empty() || !flags.is_ascii() {
        return Err(Diagnostic::error(
            "ASM-SECTION-004",
            "section name and flags must be non-empty ASCII strings",
        ));
    }
    Ok(Some((name, flags.clone())))
}

fn select_section(sections: &mut Vec<SectionState>, name: String, flags: String) -> Result<usize> {
    if let Some((index, section)) = sections
        .iter()
        .enumerate()
        .find(|(_, section)| section.name == name)
    {
        if section.flags != flags {
            return Err(Diagnostic::error(
                "ASM-SECTION-005",
                "section flags cannot change after first declaration",
            ));
        }
        return Ok(index);
    }
    sections.push(SectionState {
        name,
        flags,
        address: None,
        alignment: 1,
        bytes: Vec::new(),
    });
    Ok(sections.len() - 1)
}

fn default_sections() -> Vec<SectionState> {
    vec![SectionState {
        name: ".text".into(),
        flags: "ax".into(),
        address: None,
        alignment: 1,
        bytes: Vec::new(),
    }]
}

fn section_alignment_requirement(line: &ParsedLine, symbols: &SymbolValues) -> Result<Option<u64>> {
    match line.mnemonic.as_deref() {
        Some(".align") => {
            if line.operands.len() != 1 {
                return Ok(None);
            }
            let exponent = immediate(&line.operands[0], symbols, 0, 63)?;
            Ok(Some(1u64 << u32::try_from(exponent).unwrap()))
        }
        Some(".balign") => {
            if line.operands.len() != 1 {
                return Ok(None);
            }
            let boundary = immediate(&line.operands[0], symbols, 1, i128::from(u64::MAX))?;
            Ok(Some(u64::try_from(boundary).unwrap()))
        }
        _ => Ok(None),
    }
}

fn is_include_directive(source: &str) -> bool {
    let source = strip_macro_comment(source).trim_start();
    let Some(prefix) = source.get(..8) else {
        return false;
    };
    prefix.eq_ignore_ascii_case(".include")
        && source[8..]
            .chars()
            .next()
            .is_none_or(|character| character.is_ascii_whitespace())
}

fn include_request(source: &str) -> Result<Option<String>> {
    if !is_include_directive(source) {
        return Ok(None);
    }
    let parsed = parse_line(strip_macro_comment(source)).map_err(|error| {
        Diagnostic::error(
            "ASM-INCLUDE-002",
            format!("invalid include directive: {}", error.message),
        )
    })?;
    if parsed.mnemonic.as_deref() != Some(".include") || parsed.operands.len() != 1 {
        return Err(Diagnostic::error(
            "ASM-INCLUDE-002",
            ".include expects one quoted path",
        ));
    }
    let OperandKind::String(path) = &parsed.operands[0].kind else {
        return Err(Diagnostic::error(
            "ASM-INCLUDE-002",
            ".include expects one quoted path",
        ));
    };
    if path.is_empty() {
        return Err(Diagnostic::error(
            "ASM-INCLUDE-002",
            "include path cannot be empty",
        ));
    }
    Ok(Some(path.clone()))
}

struct IncludeLoader {
    roots: Vec<PathBuf>,
    main_base: Option<PathBuf>,
    max_depth: usize,
    max_bytes: usize,
    max_files: usize,
    total_bytes: usize,
    loaded_files: usize,
    active: Vec<PathBuf>,
}

impl IncludeLoader {
    fn new(options: &AssemblyOptions) -> Result<Self> {
        if options.max_include_depth == 0
            || options.max_include_depth > MAX_INCLUDE_DEPTH
            || options.max_include_bytes == 0
            || options.max_include_bytes > MAX_INCLUDE_BYTES
            || options.max_include_files == 0
            || options.max_include_files > MAX_INCLUDE_FILES
        {
            return Err(Diagnostic::error(
                "ASM-INCLUDE-005",
                "include limits exceed the supported safety bounds",
            ));
        }
        let mut roots = Vec::with_capacity(options.include_roots.len());
        for root in &options.include_roots {
            let canonical = fs::canonicalize(root).map_err(|error| {
                Diagnostic::error(
                    "ASM-INCLUDE-001",
                    format!("cannot access include root {}: {error}", root.display()),
                )
            })?;
            if !canonical.is_dir() {
                return Err(Diagnostic::error(
                    "ASM-INCLUDE-001",
                    format!("include root is not a directory: {}", root.display()),
                ));
            }
            roots.push(canonical);
        }
        let main_base = if let Some(source_path) = &options.source_path {
            let parent = source_path.parent().unwrap_or_else(|| Path::new("."));
            let canonical = fs::canonicalize(parent).map_err(|error| {
                Diagnostic::error(
                    "ASM-INCLUDE-001",
                    format!(
                        "cannot access source directory {}: {error}",
                        parent.display()
                    ),
                )
            })?;
            if !roots.is_empty() && !roots.iter().any(|root| canonical.starts_with(root)) {
                return Err(Diagnostic::error(
                    "ASM-INCLUDE-003",
                    "source directory is outside the configured include roots",
                ));
            }
            Some(canonical)
        } else {
            roots.first().cloned()
        };
        Ok(Self {
            roots,
            main_base,
            max_depth: options.max_include_depth,
            max_bytes: options.max_include_bytes,
            max_files: options.max_include_files,
            total_bytes: 0,
            loaded_files: 0,
            active: Vec::new(),
        })
    }

    fn resolve(
        &mut self,
        request: &str,
        current_file: Option<&Path>,
        depth: usize,
    ) -> Result<PathBuf> {
        if self.roots.is_empty() {
            return Err(Diagnostic::error(
                "ASM-INCLUDE-001",
                "includes require at least one configured include root",
            ));
        }
        if depth > self.max_depth {
            return Err(Diagnostic::error(
                "ASM-INCLUDE-005",
                "include depth quota exceeded",
            ));
        }
        let requested = Path::new(request);
        if requested.is_absolute()
            || requested
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(Diagnostic::error(
                "ASM-INCLUDE-003",
                "include path must be relative and cannot contain '..'",
            ));
        }
        let base = current_file
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| self.main_base.clone())
            .ok_or_else(|| {
                Diagnostic::error(
                    "ASM-INCLUDE-001",
                    "no base directory is available for the include",
                )
            })?;
        let candidate = base.join(requested);
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            Diagnostic::error(
                "ASM-INCLUDE-001",
                format!(
                    "cannot read included source {}: {error}",
                    candidate.display()
                ),
            )
        })?;
        if !canonical.is_file() || !self.roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(Diagnostic::error(
                "ASM-INCLUDE-003",
                format!(
                    "included source is outside the sandbox: {}",
                    candidate.display()
                ),
            ));
        }
        if self.active.iter().any(|active| active == &canonical) {
            return Err(Diagnostic::error(
                "ASM-INCLUDE-004",
                format!("cyclic include detected for {}", canonical.display()),
            ));
        }
        Ok(canonical)
    }

    fn expand_source(
        &mut self,
        source: &str,
        source_path: Option<&Path>,
    ) -> Result<Vec<ExpandedLine>> {
        let root_path = source_path.and_then(|path| fs::canonicalize(path).ok());
        if let Some(root_path) = root_path {
            self.active.push(root_path);
        }
        let result = self.expand_text(source, source_path, 0);
        if source_path.is_some() {
            self.active.pop();
        }
        result
    }

    fn expand_text(
        &mut self,
        source: &str,
        source_path: Option<&Path>,
        depth: usize,
    ) -> Result<Vec<ExpandedLine>> {
        let mut output = Vec::new();
        for (index, raw_line) in source.lines().enumerate() {
            let source_line = u32::try_from(index + 1).unwrap_or(u32::MAX);
            if let Some(request) = include_request(raw_line)? {
                let path = self.resolve(&request, source_path, depth + 1)?;
                if self.loaded_files >= self.max_files {
                    return Err(Diagnostic::error(
                        "ASM-INCLUDE-005",
                        "include file quota exceeded",
                    ));
                }
                let bytes = fs::read(&path).map_err(|error| {
                    Diagnostic::error(
                        "ASM-INCLUDE-001",
                        format!("cannot read included source {}: {error}", path.display()),
                    )
                })?;
                let length = bytes.len();
                self.total_bytes = self.total_bytes.checked_add(length).ok_or_else(|| {
                    Diagnostic::error("ASM-INCLUDE-005", "included source byte quota exceeded")
                })?;
                if self.total_bytes > self.max_bytes {
                    return Err(Diagnostic::error(
                        "ASM-INCLUDE-005",
                        "included source byte quota exceeded",
                    ));
                }
                let included = String::from_utf8(bytes).map_err(|_| {
                    Diagnostic::error(
                        "ASM-INCLUDE-006",
                        format!("included source is not valid UTF-8: {}", path.display()),
                    )
                })?;
                self.loaded_files += 1;
                self.active.push(path.clone());
                let nested = self.expand_text(&included, Some(&path), depth + 1)?;
                self.active.pop();
                output.extend(nested);
            } else {
                output.push(ExpandedLine {
                    text: raw_line.trim_end().to_owned(),
                    source_line,
                });
            }
        }
        Ok(output)
    }
}

fn strip_macro_comment(source: &str) -> &str {
    let bytes = source.as_bytes();
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            index += 1;
            continue;
        }
        if byte == b'#' || byte == b';' || (byte == b'/' && bytes.get(index + 1) == Some(&b'/')) {
            return &source[..index];
        }
        index += 1;
    }
    source
}

fn macro_words(source: &str) -> Vec<String> {
    source
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

fn valid_macro_identifier(identifier: &str) -> bool {
    let mut characters = identifier.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '.' | '$'))
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '$')
        })
}

fn macro_header(source: &str) -> Option<&str> {
    let source = strip_macro_comment(source).trim_start();
    let prefix = source.get(..6)?;
    if !prefix.eq_ignore_ascii_case(".macro") {
        return None;
    }
    let rest = &source[6..];
    if rest
        .chars()
        .next()
        .is_some_and(|character| !character.is_ascii_whitespace())
    {
        return None;
    }
    Some(rest.trim())
}

fn is_end_macro(source: &str) -> bool {
    let source = strip_macro_comment(source).trim();
    source.eq_ignore_ascii_case(".endm") || source.eq_ignore_ascii_case(".endmacro")
}

fn macro_invocation(source: &str) -> Option<(String, &str)> {
    let source = strip_macro_comment(source).trim_start();
    let end = source
        .find(|character: char| character.is_ascii_whitespace() || character == ',')
        .unwrap_or(source.len());
    let name = &source[..end];
    valid_macro_identifier(name).then(|| (name.to_ascii_lowercase(), source[end..].trim()))
}

fn macro_arguments(source: &str) -> Vec<String> {
    let source = source.trim();
    if source.is_empty() {
        return Vec::new();
    }
    if !source.contains(',') {
        return vec![source.to_owned()];
    }
    let mut arguments = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                arguments.push(source[start..index].trim().to_owned());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    arguments.push(source[start..].trim().to_owned());
    arguments
}

fn substitute_macro_line(body: &str, parameters: &[String], arguments: &[String]) -> String {
    let mut replacements: Vec<_> = parameters.iter().zip(arguments).collect();
    replacements.sort_by_key(|(parameter, _)| Reverse(parameter.len()));
    let mut result = body.to_owned();
    for (parameter, argument) in replacements {
        result = result.replace(&format!("\\{parameter}"), argument);
        result = result.replace(&format!("${parameter}"), argument);
    }
    result
}

fn expand_macro_line(
    line: ExpandedLine,
    macros: &BTreeMap<String, MacroDefinition>,
    stack: &mut Vec<String>,
    output: &mut Vec<ExpandedLine>,
) -> Result<()> {
    let Some((name, arguments_source)) = macro_invocation(&line.text) else {
        if output.len() >= MAX_EXPANDED_LINES {
            return Err(Diagnostic::error(
                "ASM-MACRO-005",
                "expanded source exceeds macro line quota",
            ));
        }
        output.push(line);
        return Ok(());
    };
    let Some(definition) = macros.get(&name) else {
        if output.len() >= MAX_EXPANDED_LINES {
            return Err(Diagnostic::error(
                "ASM-MACRO-005",
                "expanded source exceeds macro line quota",
            ));
        }
        output.push(line);
        return Ok(());
    };
    if stack.iter().any(|active| active == &name) {
        return Err(Diagnostic::error(
            "ASM-MACRO-004",
            "recursive macro invocation",
        ));
    }
    if stack.len() >= MAX_MACRO_DEPTH {
        return Err(Diagnostic::error(
            "ASM-MACRO-005",
            "macro expansion depth quota exceeded",
        ));
    }
    let arguments = macro_arguments(arguments_source);
    if arguments.len() != definition.parameters.len() {
        return Err(Diagnostic::error(
            "ASM-MACRO-003",
            "macro argument count does not match parameters",
        ));
    }
    stack.push(name.clone());
    for body_line in &definition.body {
        let expanded = ExpandedLine {
            text: substitute_macro_line(&body_line.text, &definition.parameters, &arguments),
            source_line: body_line.source_line,
        };
        expand_macro_line(expanded, macros, stack, output)?;
    }
    stack.pop();
    Ok(())
}

fn directive_rest<'a>(source: &'a str, directive: &str) -> Option<&'a str> {
    let source = strip_macro_comment(source).trim_start();
    let prefix = source.get(..directive.len())?;
    if !prefix.eq_ignore_ascii_case(directive) {
        return None;
    }
    if source[directive.len()..]
        .chars()
        .next()
        .is_some_and(|character| !character.is_ascii_whitespace())
    {
        return None;
    }
    Some(source[directive.len()..].trim())
}

fn conditional_directive(source: &str) -> Result<Option<ConditionalDirective>> {
    if let Some(expression) = directive_rest(source, ".if") {
        if expression.is_empty() {
            return Err(Diagnostic::error(
                "ASM-CONDITIONAL-002",
                ".if expects one integer expression",
            ));
        }
        return Ok(Some(ConditionalDirective::If(expression.to_owned())));
    }
    if let Some(rest) = directive_rest(source, ".else") {
        if !rest.is_empty() {
            return Err(Diagnostic::error(
                "ASM-CONDITIONAL-002",
                ".else expects no operands",
            ));
        }
        return Ok(Some(ConditionalDirective::Else));
    }
    if let Some(rest) = directive_rest(source, ".endif") {
        if !rest.is_empty() {
            return Err(Diagnostic::error(
                "ASM-CONDITIONAL-002",
                ".endif expects no operands",
            ));
        }
        return Ok(Some(ConditionalDirective::EndIf));
    }
    Ok(None)
}

fn conditional_active(frames: &[ConditionalFrame]) -> bool {
    frames.iter().all(|frame| frame.branch_active)
}

fn update_conditional_symbols(
    line: &ParsedLine,
    current_global: &mut Option<String>,
    values: &mut SymbolValues,
    equ_values: &mut SymbolValues,
    equ_names: &mut BTreeSet<String>,
) -> Result<()> {
    update_scope(&line.labels, current_global)?;
    let mut constants = BTreeMap::new();
    define_absolute_directive(
        line,
        current_global.as_deref(),
        &BTreeMap::new(),
        values,
        &mut constants,
        equ_values,
        equ_names,
    )
}

fn expand_macros(source: &str, options: &AssemblyOptions) -> Result<Vec<ExpandedLine>> {
    let mut include_loader = IncludeLoader::new(options)?;
    let included_source = include_loader.expand_source(source, options.source_path.as_deref())?;
    let mut macros = BTreeMap::new();
    let mut active: Option<(String, Vec<String>, Vec<ExpandedLine>)> = None;
    let mut skipped_macro_depth = 0usize;
    let mut conditional_frames = Vec::new();
    let mut conditional_values = SymbolValues::new();
    let mut conditional_equ_values = SymbolValues::new();
    let mut conditional_equ_names = BTreeSet::new();
    let mut conditional_global = None;
    let mut retained = Vec::new();
    for expanded_line in included_source {
        let source_line = expanded_line.source_line;
        let text = expanded_line.text;
        let control_text = strip_macro_comment(&text);
        if skipped_macro_depth != 0 {
            if macro_header(control_text).is_some() {
                skipped_macro_depth += 1;
            } else if is_end_macro(control_text) {
                skipped_macro_depth -= 1;
            }
            continue;
        }
        if active.is_some() {
            if is_end_macro(control_text) {
                let (name, parameters, body) = active.take().expect("active macro");
                macros.insert(name, MacroDefinition { parameters, body });
                continue;
            }
            if macro_header(control_text).is_some() {
                return Err(Diagnostic::error(
                    "ASM-MACRO-006",
                    "nested macro definitions are not supported",
                ));
            }
            if directive_rest(control_text, ".if").is_some()
                || directive_rest(control_text, ".else").is_some()
                || directive_rest(control_text, ".endif").is_some()
            {
                return Err(Diagnostic::error(
                    "ASM-CONDITIONAL-006",
                    "conditional directives inside macros are not supported",
                ));
            }
            let (_, _, body) = active.as_mut().expect("active macro");
            if body.len() >= MAX_MACRO_BODY_LINES {
                return Err(Diagnostic::error(
                    "ASM-MACRO-005",
                    "macro body exceeds line quota",
                ));
            }
            body.push(ExpandedLine { text, source_line });
            continue;
        }
        if let Some(directive) = conditional_directive(control_text)? {
            match directive {
                ConditionalDirective::If(expression) => {
                    if conditional_frames.len() >= MAX_CONDITIONAL_DEPTH {
                        return Err(Diagnostic::error(
                            "ASM-CONDITIONAL-005",
                            "conditional nesting depth quota exceeded",
                        ));
                    }
                    let parent_active = conditional_active(&conditional_frames);
                    let condition_true = if parent_active {
                        let scoped =
                            scoped_symbols(&conditional_values, conditional_global.as_deref());
                        expr::evaluate(&expression, &scoped)
                            .map(|value| value != 0)
                            .map_err(|error| {
                                Diagnostic::error(
                                    "ASM-CONDITIONAL-003",
                                    format!("invalid .if expression: {}", error.message),
                                )
                            })?
                    } else {
                        false
                    };
                    conditional_frames.push(ConditionalFrame {
                        parent_active,
                        condition_true,
                        branch_active: parent_active && condition_true,
                        else_seen: false,
                    });
                }
                ConditionalDirective::Else => {
                    let Some(frame) = conditional_frames.last_mut() else {
                        return Err(Diagnostic::error("ASM-CONDITIONAL-001", "unexpected .else"));
                    };
                    if frame.else_seen {
                        return Err(Diagnostic::error("ASM-CONDITIONAL-001", "duplicate .else"));
                    }
                    frame.else_seen = true;
                    frame.branch_active = frame.parent_active && !frame.condition_true;
                }
                ConditionalDirective::EndIf => {
                    if conditional_frames.pop().is_none() {
                        return Err(Diagnostic::error(
                            "ASM-CONDITIONAL-001",
                            "unexpected .endif",
                        ));
                    }
                }
            }
            continue;
        }
        if let Some(header) = macro_header(control_text) {
            if !conditional_active(&conditional_frames) {
                skipped_macro_depth = 1;
                continue;
            }
            let words = macro_words(header);
            let Some((name, parameters)) = words.split_first() else {
                return Err(Diagnostic::error("ASM-MACRO-001", "macro name is missing"));
            };
            if !valid_macro_identifier(name) || parameters.len() > MAX_MACRO_PARAMETERS {
                return Err(Diagnostic::error(
                    "ASM-MACRO-001",
                    "invalid macro name or parameter list",
                ));
            }
            let parameters: Vec<_> = parameters
                .iter()
                .map(|parameter| parameter.to_ascii_lowercase())
                .collect();
            if parameters
                .iter()
                .any(|parameter| !valid_macro_identifier(parameter))
                || parameters.windows(2).any(|pair| pair[0] == pair[1])
            {
                return Err(Diagnostic::error(
                    "ASM-MACRO-001",
                    "invalid or duplicate macro parameter",
                ));
            }
            if macros.contains_key(&name.to_ascii_lowercase()) || active.is_some() {
                return Err(Diagnostic::error(
                    "ASM-MACRO-002",
                    "duplicate macro definition",
                ));
            }
            if macros.len() >= MAX_MACRO_DEFINITIONS {
                return Err(Diagnostic::error(
                    "ASM-MACRO-005",
                    "macro definition quota exceeded",
                ));
            }
            active = Some((name.to_ascii_lowercase(), parameters, Vec::new()));
            continue;
        }
        if is_end_macro(control_text) {
            return Err(Diagnostic::error(
                "ASM-MACRO-002",
                "unexpected macro terminator",
            ));
        }
        if !conditional_active(&conditional_frames) {
            continue;
        }
        if control_text.contains(':')
            || directive_rest(control_text, ".equ").is_some()
            || directive_rest(control_text, ".set").is_some()
        {
            let parsed = parse_line(&text)?;
            update_conditional_symbols(
                &parsed,
                &mut conditional_global,
                &mut conditional_values,
                &mut conditional_equ_values,
                &mut conditional_equ_names,
            )?;
        }
        retained.push(ExpandedLine { text, source_line });
    }
    if active.is_some() {
        return Err(Diagnostic::error(
            "ASM-MACRO-002",
            "unterminated macro definition",
        ));
    }
    if skipped_macro_depth != 0 {
        return Err(Diagnostic::error(
            "ASM-MACRO-002",
            "unterminated macro definition",
        ));
    }
    if !conditional_frames.is_empty() {
        return Err(Diagnostic::error(
            "ASM-CONDITIONAL-001",
            "unterminated conditional block",
        ));
    }
    let mut output = Vec::new();
    let mut stack = Vec::new();
    for line in retained {
        expand_macro_line(line, &macros, &mut stack, &mut output)?;
    }
    Ok(output)
}

pub fn assemble_program(source: &str) -> Result<ObjectImage> {
    assemble_program_with_options(source, &AssemblyOptions::default())
}

pub fn assemble_program_with_options(
    source: &str,
    options: &AssemblyOptions,
) -> Result<ObjectImage> {
    let expanded_source = expand_macros(source, options)?;
    let lines: Vec<_> = expanded_source
        .iter()
        .map(|line| parse_line(&line.text))
        .collect::<Result<_>>()?;
    let mut symbols = BTreeMap::new();
    let mut values = SymbolValues::new();
    let mut constants = BTreeMap::new();
    let mut equ_values = SymbolValues::new();
    let mut equ_names = BTreeSet::new();
    let mut declared_sections = default_sections();
    let mut current_global = None;
    let mut pc = 0u64;
    for line in &lines {
        if let Some((name, flags)) = section_spec(line)? {
            select_section(&mut declared_sections, name, flags)?;
        }
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
    let mut listing = Vec::with_capacity(lines.len());
    let mut sections = default_sections();
    let mut current_section = 0usize;
    sections[0].address = Some(0);
    let mut emit_values: SymbolValues = symbols
        .iter()
        .map(|(name, address)| (name.clone(), i128::from(*address)))
        .collect();
    emit_values.extend(equ_values);
    let mut current_global = None;
    pc = 0;
    for (line_index, line) in lines.into_iter().enumerate() {
        update_scope(&line.labels, &mut current_global)?;
        if let Some((name, flags)) = section_spec(&line)? {
            current_section = select_section(&mut sections, name, flags)?;
            sections[current_section].address.get_or_insert(pc);
            let section_name = sections[current_section].name.clone();
            listing.push(ListingEntry {
                source_line: expanded_source[line_index].source_line,
                address: pc,
                section: section_name,
                source: expanded_source[line_index].text.clone(),
                bytes: Vec::new(),
            });
            continue;
        }
        sections[current_section].address.get_or_insert(pc);
        let section_name = sections[current_section].name.clone();
        if line.mnemonic.is_none() {
            listing.push(ListingEntry {
                source_line: expanded_source[line_index].source_line,
                address: pc,
                section: section_name,
                source: expanded_source[line_index].text.clone(),
                bytes: Vec::new(),
            });
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
            listing.push(ListingEntry {
                source_line: expanded_source[line_index].source_line,
                address: pc,
                section: section_name,
                source: expanded_source[line_index].text.clone(),
                bytes: Vec::new(),
            });
            continue;
        }
        if line.mnemonic.as_deref() == Some(".equ") {
            listing.push(ListingEntry {
                source_line: expanded_source[line_index].source_line,
                address: pc,
                section: section_name,
                source: expanded_source[line_index].text.clone(),
                bytes: Vec::new(),
            });
            continue;
        }
        let scoped = scoped_symbols(&emit_values, current_global.as_deref());
        let resolved = resolve_control_label(line, pc, &scoped)?;
        let image = assemble_parsed(&resolved, pc, &scoped)?;
        let bytes = image.text;
        if let Some(alignment) = section_alignment_requirement(&resolved, &scoped)? {
            sections[current_section].alignment =
                sections[current_section].alignment.max(alignment);
        }
        sections[current_section].bytes.extend_from_slice(&bytes);
        listing.push(ListingEntry {
            source_line: expanded_source[line_index].source_line,
            address: pc,
            section: section_name,
            source: expanded_source[line_index].text.clone(),
            bytes: bytes.clone(),
        });
        text.extend_from_slice(&bytes);
        pc += bytes.len() as u64;
    }
    let entry = symbols.get("_start").copied().unwrap_or(0);
    let sections = sections
        .into_iter()
        .map(|section| SectionImage {
            address: section.address.unwrap_or(pc),
            name: section.name,
            flags: section.flags,
            alignment: section.alignment,
            bytes: section.bytes,
        })
        .collect();
    Ok(ObjectImage {
        text,
        entry,
        symbols,
        constants,
        listing,
        sections,
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
        ".text" | ".rodata" | ".data" | ".bss" | ".section" => return Ok(0),
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn with_include_root<T>(test: impl FnOnce(&Path) -> T) -> T {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rvmonitor-assembler-include-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let result = test(&root);
        fs::remove_dir_all(&root).unwrap();
        result
    }

    fn write_include(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    #[test]
    fn assembles_required_first_program() {
        assert_eq!(
            assemble("addi x1,x0,1").unwrap().text,
            [0x93, 0x00, 0x10, 0x00]
        );
    }

    #[test]
    fn produces_a_source_address_bytes_listing() {
        let image =
            assemble_program("_start: addi x1,x0,1\n.equ VALUE, 7\n.balign 8\n.string \"ok\"")
                .unwrap();
        assert_eq!(image.listing.len(), 4);
        assert_eq!(image.listing[0].source_line, 1);
        assert_eq!(image.listing[0].address, 0);
        assert_eq!(image.listing[0].source, "_start: addi x1,x0,1");
        assert_eq!(image.listing[0].bytes, [0x93, 0x00, 0x10, 0x00]);
        assert_eq!(image.listing[1].address, 4);
        assert!(image.listing[1].bytes.is_empty());
        assert_eq!(image.listing[2].address, 4);
        assert_eq!(image.listing[2].bytes, [0, 0, 0, 0]);
        assert_eq!(image.listing[3].address, 8);
        assert_eq!(image.listing[3].bytes, [b'o', b'k', 0]);
        let listed_bytes: Vec<u8> = image
            .listing
            .iter()
            .flat_map(|entry| entry.bytes.iter().copied())
            .collect();
        assert_eq!(listed_bytes, image.text);

        let single = assemble("addi x1,x0,1").unwrap();
        assert_eq!(single.listing[0].source_line, 1);
        assert_eq!(single.listing[0].bytes, single.text);
    }

    #[test]
    fn renders_a_reproducible_text_listing() {
        let image = assemble_program(".text\n_start: addi x1,x0,1\n.equ VALUE, 7").unwrap();
        let rendered = render_listing(&image);
        assert_eq!(rendered, render_listing(&image));
        let lines: Vec<_> = rendered.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("0001 0x0000000000000000 .text"));
        assert!(lines[1].contains("93 00 10 00"));
        assert!(lines[1].ends_with("| _start: addi x1,x0,1"));
        assert!(lines[2].contains(" .equ VALUE, 7"));
        assert!(lines[2].contains(" - "));
    }

    #[test]
    fn assembles_named_sections_and_keeps_flat_image_compatibility() {
        let image = assemble_program(
            ".text\n_start: addi x1,x0,1\n.rodata\nmessage: .string \"ok\"\n.data\n.balign 8\nvalue: .word 0x1234\n.section .custom,\"aw\"\n.byte 9\n.bss",
        )
        .unwrap();
        assert_eq!(image.text.len(), 13);
        assert_eq!(image.symbols["_start"], 0);
        assert_eq!(image.symbols["message"], 4);
        assert_eq!(image.symbols["value"], 8);
        assert_eq!(image.sections.len(), 5);
        assert_eq!(image.sections[0].name, ".text");
        assert_eq!(image.sections[0].flags, "ax");
        assert_eq!(image.sections[0].address, 0);
        assert_eq!(image.sections[0].bytes, [0x93, 0x00, 0x10, 0x00]);
        assert_eq!(image.sections[1].name, ".rodata");
        assert_eq!(image.sections[1].address, 4);
        assert_eq!(image.sections[1].bytes, [b'o', b'k', 0]);
        assert_eq!(image.sections[2].name, ".data");
        assert_eq!(image.sections[2].flags, "aw");
        assert_eq!(image.sections[2].address, 7);
        assert_eq!(image.sections[2].alignment, 8);
        assert_eq!(image.sections[2].bytes, [0, 0x34, 0x12, 0, 0]);
        assert_eq!(image.sections[3].name, ".custom");
        assert_eq!(image.sections[3].address, 12);
        assert_eq!(image.sections[3].bytes, [9]);
        assert_eq!(image.sections[4].name, ".bss");
        assert_eq!(image.sections[4].address, 13);
        assert!(image.sections[4].bytes.is_empty());
        assert_eq!(image.listing[1].section, ".text");
        assert_eq!(image.listing[2].section, ".rodata");
        let section_bytes: Vec<u8> = image
            .sections
            .iter()
            .flat_map(|section| section.bytes.iter().copied())
            .collect();
        assert_eq!(section_bytes, image.text);
    }

    #[test]
    fn rejects_invalid_or_inconsistent_section_declarations() {
        let error = assemble_program(".text ax\n.byte 1").unwrap_err();
        assert_eq!(error.code, "ASM-SECTION-001");
        let error = assemble_program(".section .custom,\"a\"\n.byte 1\n.section .custom,\"aw\"")
            .unwrap_err();
        assert_eq!(error.code, "ASM-SECTION-005");
        let error = assemble_program(".section .custom, aw").unwrap_err();
        assert_eq!(error.code, "ASM-SECTION-003");
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
    fn assembles_exact_float_data_literals_for_all_binary_formats() {
        assert_eq!(
            assemble(".binary16 bits16(0x3e00),1.5").unwrap().text,
            [0x00, 0x3e, 0x00, 0x3e]
        );
        assert_eq!(
            assemble(".float bits32(0x3fc00000),1.5").unwrap().text,
            [0x00, 0x00, 0xc0, 0x3f, 0x00, 0x00, 0xc0, 0x3f]
        );
        assert_eq!(
            assemble(".double bits64(0x3ff8000000000000),1.5")
                .unwrap()
                .text,
            [
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0xf8, 0x3f
            ]
        );
        let binary128 = 0x3fff_8000_0000_0000_0000_0000_0000_0000u128;
        assert_eq!(
            assemble(".binary128 bits128(0x3fff8000000000000000000000000000)")
                .unwrap()
                .text,
            binary128.to_le_bytes()
        );
    }

    #[test]
    fn rejects_float_data_width_mismatch_and_unsupported_binary128_decimal() {
        assert_eq!(
            assemble(".binary16 bits32(0x3f800000)").unwrap_err().code,
            "ASM-FLOAT-002"
        );
        assert_eq!(
            assemble(".binary128 1.5").unwrap_err().code,
            "ASM-FLOAT-001"
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

    #[test]
    fn expands_parameterized_macros_and_keeps_body_source_lines() {
        let image = assemble_program(
            ".macro inc rd\n  addi \\rd,\\rd,1\n.endm\n_start: addi x1,x0,0\ninc x1",
        )
        .unwrap();
        assert_eq!(image.text, [0x93, 0x00, 0x00, 0x00, 0x93, 0x80, 0x10, 0x00]);
        assert_eq!(image.listing.last().unwrap().source_line, 2);
        assert_eq!(image.listing.last().unwrap().source, "  addi x1,x1,1");
    }

    #[test]
    fn expands_nested_macros_with_comma_arguments() {
        let image = assemble_program(
            ".macro inc rd\n  addi \\rd,\\rd,1\n.endm\n.macro twice rd\n  inc \\rd\n  inc \\rd\n.endm\ntwice x1",
        )
        .unwrap();
        assert_eq!(image.text.len(), 8);

        let image =
            assemble_program(".macro load rd, value\n  addi \\rd,x0,\\value\n.endm\nload x2, 7")
                .unwrap();
        assert_eq!(image.text, [0x13, 0x01, 0x70, 0x00]);
    }

    #[test]
    fn rejects_invalid_macro_structure_and_expansion() {
        let error = assemble_program(".macro inc rd\n addi \\rd,\\rd,1\n.endm\ninc").unwrap_err();
        assert_eq!(error.code, "ASM-MACRO-003");

        let error = assemble_program(".macro loop\n loop\n.endm\nloop").unwrap_err();
        assert_eq!(error.code, "ASM-MACRO-004");

        let error = assemble_program(".macro broken\n addi x1,x0,1").unwrap_err();
        assert_eq!(error.code, "ASM-MACRO-002");
    }

    #[test]
    fn expands_nested_includes_relative_to_the_including_file() {
        with_include_root(|root| {
            write_include(root, "lib/constants.s", ".equ VALUE, 1\n");
            write_include(
                root,
                "lib/macros.s",
                ".include \"constants.s\"\n.macro load rd\n  addi \\rd,x0,VALUE\n.endm\n",
            );
            let options = AssemblyOptions {
                include_roots: vec![root.to_path_buf()],
                ..AssemblyOptions::default()
            };
            let image =
                assemble_program_with_options(".include \"lib/macros.s\"\nload x1", &options)
                    .unwrap();
            assert_eq!(image.text, [0x93, 0x00, 0x10, 0x00]);
            assert_eq!(image.listing.last().unwrap().source_line, 3);
            assert_eq!(image.listing.last().unwrap().source, "  addi x1,x0,VALUE");
        });
    }

    #[test]
    fn rejects_includes_without_sandbox_traversal_cycles_and_quota_overflow() {
        let error = assemble_program(".include \"missing.s\"").unwrap_err();
        assert_eq!(error.code, "ASM-INCLUDE-001");

        with_include_root(|root| {
            write_include(root, "escape.s", "addi x1,x0,1\n");
            let options = AssemblyOptions {
                include_roots: vec![root.to_path_buf()],
                ..AssemblyOptions::default()
            };
            let error =
                assemble_program_with_options(".include \"../escape.s\"", &options).unwrap_err();
            assert_eq!(error.code, "ASM-INCLUDE-003");

            write_include(root, "a.s", ".include \"b.s\"\n");
            write_include(root, "b.s", ".include \"a.s\"\n");
            let error = assemble_program_with_options(".include \"a.s\"", &options).unwrap_err();
            assert_eq!(error.code, "ASM-INCLUDE-004");

            write_include(root, "large.s", "addi x1,x0,1\n");
            let options = AssemblyOptions {
                include_roots: vec![root.to_path_buf()],
                max_include_bytes: 1,
                ..AssemblyOptions::default()
            };
            let error =
                assemble_program_with_options(".include \"large.s\"", &options).unwrap_err();
            assert_eq!(error.code, "ASM-INCLUDE-005");
        });
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_that_escapes_the_include_root() {
        use std::os::unix::fs::symlink;

        with_include_root(|root| {
            let outside = root.with_file_name(format!(
                "rvmonitor-assembler-outside-{}",
                std::process::id()
            ));
            fs::create_dir_all(&outside).unwrap();
            write_include(&outside, "secret.s", "addi x1,x0,1\n");
            symlink(outside.join("secret.s"), root.join("link.s")).unwrap();
            let options = AssemblyOptions {
                include_roots: vec![root.to_path_buf()],
                ..AssemblyOptions::default()
            };
            let error = assemble_program_with_options(".include \"link.s\"", &options).unwrap_err();
            assert_eq!(error.code, "ASM-INCLUDE-003");
            fs::remove_dir_all(outside).unwrap();
        });
    }

    #[test]
    fn selects_conditional_branches_using_sequential_constants() {
        let image = assemble_program(
            ".equ ENABLE, 1\n.if ENABLE\n  .set VALUE, 7\n  addi x1,x0,VALUE\n.else\n  addi x1,x0,99\n.endif",
        )
        .unwrap();
        assert_eq!(image.text, [0x93, 0x00, 0x70, 0x00]);
        assert_eq!(image.constants["VALUE"], 7);

        let image = assemble_program(
            ".equ ENABLE, 0\n.if ENABLE\n  unknown x1,x2,x3\n.else\n  addi x1,x0,2\n.endif",
        )
        .unwrap();
        assert_eq!(image.text, [0x93, 0x00, 0x20, 0x00]);
    }

    #[test]
    fn supports_nested_conditionals_and_ignores_dead_expressions() {
        let image = assemble_program(
            ".if 0\n  .if UNKNOWN_SYMBOL\n    invalid x1\n  .endif\n.else\n  .if (1 + 1)\n    addi x1,x0,3\n  .endif\n.endif",
        )
        .unwrap();
        assert_eq!(image.text, [0x93, 0x00, 0x30, 0x00]);
    }

    #[test]
    fn rejects_invalid_conditional_structure_and_macro_mixture() {
        let error = assemble_program(".else").unwrap_err();
        assert_eq!(error.code, "ASM-CONDITIONAL-001");

        let error = assemble_program(".if UNKNOWN_SYMBOL\naddi x1,x0,1\n.endif").unwrap_err();
        assert_eq!(error.code, "ASM-CONDITIONAL-003");

        let error = assemble_program(".macro broken\n.if 1\n addi x1,x0,1\n.endif\n.endm\nbroken")
            .unwrap_err();
        assert_eq!(error.code, "ASM-CONDITIONAL-006");

        let error = assemble_program(".if 1\naddi x1,x0,1").unwrap_err();
        assert_eq!(error.code, "ASM-CONDITIONAL-001");
    }

    #[test]
    fn enforces_conditional_nesting_quota() {
        let mut source = String::new();
        for _ in 0..=MAX_CONDITIONAL_DEPTH {
            source.push_str(".if 1\n");
        }
        source.push_str("addi x1,x0,1\n");
        for _ in 0..=MAX_CONDITIONAL_DEPTH {
            source.push_str(".endif\n");
        }
        let error = assemble_program(&source).unwrap_err();
        assert_eq!(error.code, "ASM-CONDITIONAL-005");
    }
}
