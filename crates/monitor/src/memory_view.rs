use std::fmt::Write;

use luna_diag::{Diagnostic, Result};

pub(crate) const BYTES_PER_ROW: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextEncoding {
    Ascii,
    Cp437,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct MemoryRange {
    /// Inclusive start, exclusive end.
    pub(crate) start: u64,
    pub(crate) end: u64,
}

#[allow(dead_code)]
impl MemoryRange {
    pub(crate) fn new(start: u64, end: u64) -> Result<Self> {
        if end < start {
            return Err(Diagnostic::error(
                "MON-MEM-VIEW-001",
                "memory selection range is reversed",
            ));
        }
        Ok(Self { start, end })
    }

    pub(crate) fn len(self) -> u64 {
        self.end - self.start
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct MemoryViewModel {
    anchor: u64,
    cursor: u64,
    selection: Option<MemoryRange>,
}

#[allow(dead_code)]
impl MemoryViewModel {
    pub(crate) const fn new(address: u64) -> Self {
        Self {
            anchor: address,
            cursor: address,
            selection: None,
        }
    }

    pub(crate) const fn anchor(self) -> u64 {
        self.anchor
    }

    pub(crate) const fn cursor(self) -> u64 {
        self.cursor
    }

    pub(crate) const fn selection(self) -> Option<MemoryRange> {
        self.selection
    }

    pub(crate) fn jump(&mut self, address: u64) {
        self.anchor = address;
        self.cursor = address;
        self.selection = None;
    }

    pub(crate) fn move_cursor(&mut self, delta: i64) -> Result<u64> {
        let next = if delta.is_negative() {
            self.cursor.checked_sub(delta.unsigned_abs())
        } else {
            self.cursor.checked_add(delta as u64)
        }
        .ok_or_else(|| Diagnostic::error("MON-MEM-VIEW-002", "memory cursor address overflow"))?;
        self.cursor = next;
        Ok(next)
    }

    pub(crate) fn select_to(&mut self, address: u64) -> Result<MemoryRange> {
        let range = if address < self.cursor {
            MemoryRange::new(address, self.cursor)?
        } else {
            MemoryRange::new(self.cursor, address)?
        };
        self.selection = Some(range);
        Ok(range)
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selection = None;
    }
}

pub(crate) fn render_hex_ascii(
    address: u64,
    bytes: &[u8],
    code: &'static str,
    encoding: TextEncoding,
) -> Result<String> {
    let mut output = String::new();
    for (row, chunk) in bytes.chunks(BYTES_PER_ROW).enumerate() {
        let row_address = address
            .checked_add((row * BYTES_PER_ROW) as u64)
            .ok_or_else(|| Diagnostic::error(code, "address overflow"))?;
        write!(output, "0x{row_address:016x}: ").unwrap();
        for byte in chunk {
            write!(output, "{byte:02x} ").unwrap();
        }
        for _ in chunk.len()..BYTES_PER_ROW {
            output.push_str("   ");
        }
        output.push('|');
        for byte in chunk {
            output.push(render_text_byte(*byte, encoding));
        }
        output.push('|');
        output.push('\n');
    }
    Ok(output.trim_end().into())
}

fn render_text_byte(byte: u8, encoding: TextEncoding) -> char {
    match encoding {
        TextEncoding::Ascii => {
            if (0x20..=0x7e).contains(&byte) {
                byte as char
            } else {
                '.'
            }
        }
        TextEncoding::Cp437 => {
            if (0x20..=0x7e).contains(&byte) {
                return byte as char;
            }
            if byte == 0x7f {
                return '⌂';
            }
            const EXTENDED: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ";
            EXTENDED
                .chars()
                .nth(usize::from(byte - 0x80))
                .unwrap_or('·')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_navigation_is_checked_and_jump_clears_selection() {
        let mut view = MemoryViewModel::new(0x10);
        assert_eq!(view.move_cursor(4).unwrap(), 0x14);
        assert_eq!(view.move_cursor(-2).unwrap(), 0x12);
        assert_eq!(
            view.move_cursor(-0x13).unwrap_err().code,
            "MON-MEM-VIEW-002"
        );
        view.select_to(0x20).unwrap();
        assert_eq!(view.selection().unwrap().len(), 0x0e);
        view.jump(0x80);
        assert_eq!(view.anchor(), 0x80);
        assert_eq!(view.cursor(), 0x80);
        assert_eq!(view.selection(), None);
    }

    #[test]
    fn selection_is_normalized_to_inclusive_exclusive_range() {
        let mut view = MemoryViewModel::new(0x20);
        view.move_cursor(0x10).unwrap();
        let range = view.select_to(0x08).unwrap();
        assert_eq!(
            range,
            MemoryRange {
                start: 0x08,
                end: 0x30
            }
        );
        view.clear_selection();
        assert_eq!(view.selection(), None);
    }

    #[test]
    fn hex_ascii_rendering_is_shared_and_byte_exact() {
        let rendered = render_hex_ascii(0x10, b"A\nxyz", "TEST-MEM", TextEncoding::Ascii).unwrap();
        assert_eq!(
            rendered,
            "0x0000000000000010: 41 0a 78 79 7a                                  |A.xyz|"
        );
    }

    #[test]
    fn cp437_rendering_preserves_bytes_and_maps_extended_glyphs() {
        let rendered = render_hex_ascii(
            0,
            &[0x41, 0x82, 0xb3, 0xff],
            "TEST-MEM",
            TextEncoding::Cp437,
        )
        .unwrap();
        assert!(rendered.contains("|Aé│ |"));
        assert!(rendered.contains("41 82 b3 ff"));
    }
}
