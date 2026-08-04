use core::arch::asm;

const LINE_CAPACITY: usize = 96;
const PROGRAM_CAPACITY: usize = 256;
const FOR_CAPACITY: usize = 8;
const INPUT_CAPACITY: usize = 96;
const VARIABLE_CAPACITY: usize = 64;
const VARIABLE_NAME_CAPACITY: usize = 16;

const SERVICE_WRITE_CHAR: u64 = 1;
const SERVICE_READ_CHAR: u64 = 2;
const SERVICE_EXIT: u64 = 3;
const SERVICE_POLL_CHAR: u64 = 5;

#[derive(Clone, Copy)]
struct ProgramLine {
    number: u16,
    length: u8,
    bytes: [u8; LINE_CAPACITY],
    used: bool,
}

impl ProgramLine {
    const fn empty() -> Self {
        Self {
            number: 0,
            length: 0,
            bytes: [0; LINE_CAPACITY],
            used: false,
        }
    }
}

#[derive(Clone, Copy)]
struct ForFrame {
    variable: usize,
    limit: f64,
    step: f64,
    body_index: usize,
    active: bool,
}

#[derive(Clone, Copy)]
struct VariableSlot {
    name: [u8; VARIABLE_NAME_CAPACITY],
    length: u8,
    value: f64,
    used: bool,
}

impl VariableSlot {
    const fn empty() -> Self {
        Self {
            name: [0; VARIABLE_NAME_CAPACITY],
            length: 0,
            value: 0.0,
            used: false,
        }
    }
}

struct Variables {
    slots: [VariableSlot; VARIABLE_CAPACITY],
}

impl Variables {
    const fn new() -> Self {
        Self {
            slots: [VariableSlot::empty(); VARIABLE_CAPACITY],
        }
    }

    fn clear(&mut self) {
        self.slots = [VariableSlot::empty(); VARIABLE_CAPACITY];
    }

    fn reset_values(&mut self) {
        for slot in &mut self.slots {
            if slot.used {
                slot.value = 0.0;
            }
        }
    }

    fn ensure(&mut self, name: &[u8]) -> Option<usize> {
        if name.is_empty() || name.len() > VARIABLE_NAME_CAPACITY {
            return None;
        }
        if let Some(index) = self.find(name) {
            return Some(index);
        }
        let index = self.slots.iter().position(|slot| !slot.used)?;
        let slot = &mut self.slots[index];
        slot.name = [0; VARIABLE_NAME_CAPACITY];
        for (destination, source) in slot.name.iter_mut().zip(name.iter()) {
            *destination = source.to_ascii_uppercase();
        }
        slot.length = name.len() as u8;
        slot.value = 0.0;
        slot.used = true;
        Some(index)
    }

    fn find(&self, name: &[u8]) -> Option<usize> {
        self.slots.iter().position(|slot| {
            slot.used
                && usize::from(slot.length) == name.len()
                && slot.name[..usize::from(slot.length)].eq_ignore_ascii_case(name)
        })
    }
}

impl ForFrame {
    const fn empty() -> Self {
        Self {
            variable: 0,
            limit: 0.0,
            step: 0.0,
            body_index: 0,
            active: false,
        }
    }
}

struct BasicState {
    lines: [ProgramLine; PROGRAM_CAPACITY],
    variables: Variables,
    for_stack: [ForFrame; FOR_CAPACITY],
    trace: bool,
}

impl BasicState {
    const fn new() -> Self {
        Self {
            lines: [ProgramLine::empty(); PROGRAM_CAPACITY],
            variables: Variables::new(),
            for_stack: [ForFrame::empty(); FOR_CAPACITY],
            trace: false,
        }
    }

    fn clear(&mut self) {
        for line in &mut self.lines {
            line.used = false;
        }
        self.variables.clear();
        self.for_stack = [ForFrame::empty(); FOR_CAPACITY];
        self.trace = false;
    }

    fn line_at(&self, index: usize) -> Option<&ProgramLine> {
        self.lines.iter().filter(|line| line.used).nth(index)
    }

    fn insert(&mut self, number: u16, text: &[u8]) -> Result<(), Error> {
        if text.len() > LINE_CAPACITY {
            return Err(Error::LineTooLong);
        }
        let mut slot = None;
        for index in 0..PROGRAM_CAPACITY {
            if self.lines[index].used && self.lines[index].number == number {
                slot = Some(index);
                break;
            }
            if !self.lines[index].used && slot.is_none() {
                slot = Some(index);
            }
        }
        let Some(index) = slot else {
            return Err(Error::ProgramFull);
        };
        if text.is_empty() {
            self.lines[index].used = false;
            return Ok(());
        }
        self.lines[index].number = number;
        self.lines[index].length = text.len() as u8;
        self.lines[index].bytes = [0; LINE_CAPACITY];
        self.lines[index].bytes[..text.len()].copy_from_slice(text);
        self.lines[index].used = true;
        self.sort();
        Ok(())
    }

    fn sort(&mut self) {
        for left in 0..PROGRAM_CAPACITY {
            for right in (left + 1)..PROGRAM_CAPACITY {
                if self.lines[right].used
                    && (!self.lines[left].used
                        || self.lines[right].number < self.lines[left].number)
                {
                    self.lines.swap(left, right);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Error {
    Syntax,
    LineTooLong,
    ProgramFull,
    DivisionByZero,
    MissingTarget,
    ForStack,
    StepZero,
    Input,
    Interrupt,
}

#[unsafe(link_section = ".payload_data")]
static mut STATE: BasicState = BasicState::new();

#[unsafe(no_mangle)]
pub extern "C" fn minibasic_entry() -> ! {
    let state = unsafe { &mut *core::ptr::addr_of_mut!(STATE) };
    state.clear();
    write_text("\r\nMiniBASIC-RV\r\nREADY> ");
    repl(state)
}

fn repl(state: &mut BasicState) -> ! {
    let mut input = [0u8; INPUT_CAPACITY];
    loop {
        let length = read_line(&mut input);
        if length == 0 {
            write_text("READY> ");
            continue;
        }
        if let Some((number, rest)) = leading_line_number(&input[..length]) {
            let result = state.insert(number, rest);
            if let Err(error) = result {
                print_error(error, Some(number), 1);
            }
            write_text("READY> ");
            continue;
        }
        let command = &input[..length];
        let result = execute_direct(state, command);
        if let Err(error) = result {
            print_error(error, None, 1);
        }
        write_text("READY> ");
    }
}

fn execute_direct(state: &mut BasicState, input: &[u8]) -> Result<(), Error> {
    if equals_word(input, b"NEW") {
        state.clear();
        return Ok(());
    }
    if equals_word(input, b"LIST") {
        list_program(state);
        return Ok(());
    }
    if equals_word(input, b"RUN") {
        return run_program(state);
    }
    if equals_word(input, b"TRACE ON") {
        state.trace = true;
        return Ok(());
    }
    if equals_word(input, b"TRACE OFF") {
        state.trace = false;
        return Ok(());
    }
    if equals_word(input, b"DUMP") {
        dump_program(state);
        return Ok(());
    }
    if equals_word(input, b"BYE") || equals_word(input, b"EXIT") {
        unsafe {
            service_call(SERVICE_EXIT, 0);
        }
        return Ok(());
    }
    if starts_word(input, b"PRINT") || input.first() == Some(&b'?') {
        let rest = if input.first() == Some(&b'?') {
            &input[1..]
        } else {
            &input[5..]
        };
        return execute_print(state, rest.trim_ascii());
    }
    execute_statement(state, input, 0).map(|_| ())
}

fn list_program(state: &BasicState) {
    for line in state.lines.iter().filter(|line| line.used) {
        print_u64(u64::from(line.number));
        write_char(b' ');
        write_bytes(&line.bytes[..usize::from(line.length)]);
        write_text("\r\n");
    }
}

fn dump_program(state: &BasicState) {
    for (index, line) in state.lines.iter().filter(|line| line.used).enumerate() {
        write_text("slot=");
        print_u64(index as u64);
        write_text(" address=0x");
        print_hex(
            state as *const BasicState as u64
                + (index * core::mem::size_of::<ProgramLine>()) as u64,
        );
        write_text(" line=");
        print_u64(u64::from(line.number));
        write_text(" length=");
        print_u64(u64::from(line.length));
        write_text(" bytes=");
        for byte in &line.bytes[..usize::from(line.length)] {
            print_hex_byte(*byte);
        }
        write_text("\r\n");
    }
    write_text("variables:\r\n");
    for slot in state.variables.slots.iter().filter(|slot| slot.used) {
        write_bytes(&slot.name[..usize::from(slot.length)]);
        write_text("=0x");
        print_hex(slot.value.to_bits());
        write_text(" (");
        print_fixed(slot.value);
        write_text(")\r\n");
    }
}

fn run_program(state: &mut BasicState) -> Result<(), Error> {
    write_text("RV64 MINIBASIC\r\n");
    state.variables.reset_values();
    state.for_stack = [ForFrame::empty(); FOR_CAPACITY];
    let mut index = 0usize;
    let mut steps = 0u64;
    while index < PROGRAM_CAPACITY {
        let Some(line) = state.line_at(index) else {
            break;
        };
        let number = line.number;
        let mut bytes = [0u8; LINE_CAPACITY];
        let length = usize::from(line.length);
        bytes[..length].copy_from_slice(&line.bytes[..length]);
        if state.trace {
            write_char(b'[');
            print_u64(u64::from(number));
            write_text("]\r\n");
        }
        match execute_statement(state, &bytes[..length], index + 1) {
            Ok(Control::Next) => index += 1,
            Ok(Control::Jump(target)) => index = target,
            Ok(Control::Stop) => return Ok(()),
            Err(error) => {
                print_error(error, Some(number), 1);
                return Ok(());
            }
        }
        steps += 1;
        if steps & 0x3f == 0 && poll_char() == Some(3) {
            return Err(Error::Interrupt);
        }
        if steps > 1_000_000 {
            return Err(Error::Interrupt);
        }
    }
    Ok(())
}

enum Control {
    Next,
    Jump(usize),
    Stop,
}

fn execute_statement(
    state: &mut BasicState,
    input: &[u8],
    next_index: usize,
) -> Result<Control, Error> {
    let input = input.trim_ascii();
    if input.is_empty() || starts_word(input, b"REM") {
        return Ok(Control::Next);
    }
    if equals_word(input, b"END") {
        return Ok(Control::Stop);
    }
    if starts_word(input, b"PRINT") || input.first() == Some(&b'?') {
        let rest = if input.first() == Some(&b'?') {
            &input[1..]
        } else {
            &input[5..]
        };
        execute_print(state, rest.trim_ascii())?;
        return Ok(Control::Next);
    }
    if starts_word(input, b"INPUT") {
        let name = input[5..].trim_ascii();
        let variable = parse_identifier(name)
            .and_then(|(_, end)| (end == name.len()).then_some(&name[..end]))
            .and_then(|name| state.variables.ensure(name))
            .ok_or(Error::Syntax)?;
        write_text("? ");
        let mut line = [0u8; INPUT_CAPACITY];
        let length = read_line(&mut line);
        let mut parser = ExprParser::new(&line[..length], &mut state.variables);
        let value = parser.parse_value().map_err(|_| Error::Input)?;
        state.variables.slots[variable].value = value;
        return Ok(Control::Next);
    }
    if starts_word(input, b"GOTO") {
        let target = parse_u16(input[4..].trim_ascii()).ok_or(Error::Syntax)?;
        return Ok(Control::Jump(
            find_line(state, target).ok_or(Error::MissingTarget)?,
        ));
    }
    if starts_word(input, b"IF") {
        let rest = input[2..].trim_ascii();
        let Some(then_at) = find_word(rest, b"THEN") else {
            return Err(Error::Syntax);
        };
        let mut parser = ExprParser::new(&rest[..then_at], &mut state.variables);
        let condition = parser.parse_value()?;
        let target = parse_u16(rest[then_at + 4..].trim_ascii()).ok_or(Error::Syntax)?;
        if condition != 0.0 {
            return Ok(Control::Jump(
                find_line(state, target).ok_or(Error::MissingTarget)?,
            ));
        }
        return Ok(Control::Next);
    }
    if starts_word(input, b"FOR") {
        return execute_for(state, input[3..].trim_ascii(), next_index);
    }
    if starts_word(input, b"NEXT") {
        let name = input[4..].trim_ascii();
        let variable = parse_identifier(name)
            .and_then(|(_, end)| (end == name.len()).then_some(&name[..end]))
            .and_then(|name| state.variables.find(name))
            .ok_or(Error::Syntax)?;
        let Some(frame_index) = (0..FOR_CAPACITY).rev().find(|index| {
            state.for_stack[*index].active && state.for_stack[*index].variable == variable
        }) else {
            return Err(Error::ForStack);
        };
        let frame = state.for_stack[frame_index];
        state.variables.slots[variable].value += frame.step;
        let continue_loop = if frame.step >= 0.0 {
            state.variables.slots[variable].value <= frame.limit
        } else {
            state.variables.slots[variable].value >= frame.limit
        };
        if continue_loop {
            return Ok(Control::Jump(frame.body_index));
        }
        state.for_stack[frame_index].active = false;
        return Ok(Control::Next);
    }
    let assignment = input.strip_prefix(b"LET ").unwrap_or(input);
    let Some(equal) = assignment.iter().position(|byte| *byte == b'=') else {
        return Err(Error::Syntax);
    };
    let name = assignment[..equal].trim_ascii();
    let variable = parse_identifier(name)
        .and_then(|(_, end)| (end == name.len()).then_some(&name[..end]))
        .and_then(|name| state.variables.ensure(name))
        .ok_or(Error::Syntax)?;
    let mut parser = ExprParser::new(&assignment[equal + 1..], &mut state.variables);
    state.variables.slots[variable].value = parser.parse_value()?;
    Ok(Control::Next)
}

fn execute_for(state: &mut BasicState, input: &[u8], body_index: usize) -> Result<Control, Error> {
    let Some(equal) = input.iter().position(|byte| *byte == b'=') else {
        return Err(Error::Syntax);
    };
    let name = input[..equal].trim_ascii();
    let variable = parse_identifier(name)
        .and_then(|(_, end)| (end == name.len()).then_some(&name[..end]))
        .and_then(|name| state.variables.ensure(name))
        .ok_or(Error::Syntax)?;
    let Some(to_at) = find_word(&input[equal + 1..], b"TO") else {
        return Err(Error::Syntax);
    };
    let to_at = to_at + equal + 1;
    let mut start_parser = ExprParser::new(&input[equal + 1..to_at], &mut state.variables);
    let start = start_parser.parse_value()?;
    let step_at = find_word(&input[to_at + 2..], b"STEP");
    let limit_end = step_at.map(|at| to_at + 2 + at).unwrap_or(input.len());
    let mut limit_parser = ExprParser::new(&input[to_at + 2..limit_end], &mut state.variables);
    let limit = limit_parser.parse_value()?;
    let mut step = 1.0;
    if let Some(step_at) = step_at {
        let mut step_parser =
            ExprParser::new(&input[to_at + 2 + step_at + 4..], &mut state.variables);
        step = step_parser.parse_value()?;
    }
    if step == 0.0 {
        return Err(Error::StepZero);
    }
    let Some(slot) = state.for_stack.iter_mut().find(|frame| !frame.active) else {
        return Err(Error::ForStack);
    };
    state.variables.slots[variable].value = start;
    *slot = ForFrame {
        variable,
        limit,
        step,
        body_index,
        active: true,
    };
    Ok(Control::Next)
}

fn execute_print(state: &mut BasicState, input: &[u8]) -> Result<(), Error> {
    let mut rest = input.trim_ascii();
    let mut first = true;
    while !rest.is_empty() {
        let separator = rest
            .iter()
            .position(|byte| *byte == b',')
            .unwrap_or(rest.len());
        let item = rest[..separator].trim_ascii();
        if !first {
            write_char(b' ');
        }
        first = false;
        if item.len() >= 2 && item[0] == b'"' && item[item.len() - 1] == b'"' {
            write_bytes(&item[1..item.len() - 1]);
        } else {
            let mut parser = ExprParser::new(item, &mut state.variables);
            print_fixed(parser.parse_value()?);
        }
        if separator == rest.len() {
            break;
        }
        rest = rest[separator + 1..].trim_ascii();
    }
    write_text("\r\n");
    Ok(())
}

struct ExprParser<'a> {
    input: &'a [u8],
    position: usize,
    variables: &'a mut Variables,
}

impl<'a> ExprParser<'a> {
    fn new(input: &'a [u8], variables: &'a mut Variables) -> Self {
        Self {
            input,
            position: 0,
            variables,
        }
    }

    fn parse_value(&mut self) -> Result<f64, Error> {
        let value = self.parse_comparison()?;
        self.skip_spaces();
        if self.position != self.input.len() {
            return Err(Error::Syntax);
        }
        Ok(value)
    }

    fn parse_comparison(&mut self) -> Result<f64, Error> {
        let left = self.parse_sum()?;
        self.skip_spaces();
        let operator = if self.take(b"<>") {
            Some(0)
        } else if self.take(b"<=") {
            Some(1)
        } else if self.take(b">=") {
            Some(2)
        } else if self.take(b"=") {
            Some(3)
        } else if self.take(b"<") {
            Some(4)
        } else if self.take(b">") {
            Some(5)
        } else {
            None
        };
        let Some(operator) = operator else {
            return Ok(left);
        };
        let right = self.parse_sum()?;
        let result = match operator {
            0 => left != right,
            1 => left <= right,
            2 => left >= right,
            3 => left == right,
            4 => left < right,
            _ => left > right,
        };
        Ok(if result { 1.0 } else { 0.0 })
    }

    fn parse_sum(&mut self) -> Result<f64, Error> {
        let mut value = self.parse_product()?;
        loop {
            self.skip_spaces();
            if self.take(b"+") {
                value += self.parse_product()?;
            } else if self.take(b"-") {
                value -= self.parse_product()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_product(&mut self) -> Result<f64, Error> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_spaces();
            if self.take(b"*") {
                value *= self.parse_factor()?;
            } else if self.take(b"/") {
                let divisor = self.parse_factor()?;
                if divisor == 0.0 {
                    return Err(Error::DivisionByZero);
                }
                value = minibasic_divide(value, divisor);
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_factor(&mut self) -> Result<f64, Error> {
        self.skip_spaces();
        if self.take(b"+") {
            return self.parse_factor();
        }
        if self.take(b"-") {
            return Ok(-self.parse_factor()?);
        }
        if self.take(b"(") {
            let value = self.parse_comparison()?;
            if !self.take(b")") {
                return Err(Error::Syntax);
            }
            return Ok(value);
        }
        if self
            .input
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        {
            let start = self.position;
            self.position += 1;
            while self.position < self.input.len()
                && (self.input[self.position].is_ascii_alphanumeric()
                    || self.input[self.position] == b'_')
            {
                self.position += 1;
            }
            let variable = self
                .variables
                .ensure(&self.input[start..self.position])
                .ok_or(Error::Syntax)?;
            return Ok(self.variables.slots[variable].value);
        }
        let start = self.position;
        while self.position < self.input.len()
            && (self.input[self.position].is_ascii_digit() || self.input[self.position] == b'.')
        {
            self.position += 1;
        }
        if start == self.position {
            return Err(Error::Syntax);
        }
        parse_decimal(&self.input[start..self.position]).ok_or(Error::Syntax)
    }

    fn take(&mut self, token: &[u8]) -> bool {
        self.skip_spaces();
        if self.input.get(self.position..self.position + token.len()) == Some(token) {
            self.position += token.len();
            true
        } else {
            false
        }
    }

    fn skip_spaces(&mut self) {
        while self.input.get(self.position) == Some(&b' ') {
            self.position += 1;
        }
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn minibasic_divide(left: f64, right: f64) -> f64 {
    left / right
}

fn leading_line_number(input: &[u8]) -> Option<(u16, &[u8])> {
    let mut end = 0;
    while end < input.len() && input[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 || input.get(end).is_some_and(|byte| *byte != b' ') {
        return None;
    }
    let rest = if end == input.len() {
        &[]
    } else {
        input[end + 1..].trim_ascii()
    };
    Some((parse_u16(&input[..end])?, rest))
}

fn parse_u16(input: &[u8]) -> Option<u16> {
    let mut value = 0u16;
    for byte in input {
        value = value.checked_mul(10)?.checked_add(u16::from(byte - b'0'))?;
    }
    Some(value)
}

fn parse_decimal(input: &[u8]) -> Option<f64> {
    let mut value = 0.0;
    let mut fraction = 0.0;
    let mut after_dot = false;
    let mut digits = false;
    for byte in input {
        if *byte == b'.' && !after_dot {
            after_dot = true;
            continue;
        }
        if !byte.is_ascii_digit() {
            return None;
        }
        digits = true;
        if after_dot {
            fraction = (fraction * 10.0 + f64::from(*byte - b'0')) / 10.0;
        } else {
            value = value * 10.0 + f64::from(*byte - b'0');
        }
    }
    digits.then_some(value + fraction)
}

fn print_fixed(value: f64) {
    if value.is_nan() {
        write_text("NAN");
        return;
    }
    if value.is_infinite() {
        write_text(if value.is_sign_negative() {
            "-INF"
        } else {
            "INF"
        });
        return;
    }
    if value.is_sign_negative() {
        write_char(b'-');
    }
    let magnitude = value.abs();
    print_u64(magnitude as u64);
    write_char(b'.');
    let mut fraction = ((magnitude - (magnitude as u64 as f64)) * 1_000_000.0 + 0.5) as u64;
    if fraction >= 1_000_000 {
        fraction = 0;
    }
    let mut divisor = 100_000;
    while divisor != 0 {
        write_char(b'0' + ((fraction / divisor) % 10) as u8);
        divisor /= 10;
    }
}

fn print_error(error: Error, line: Option<u16>, column: u64) {
    let code = match error {
        Error::Syntax => "BASIC-SYNTAX-001",
        Error::LineTooLong => "BASIC-MEM-001",
        Error::ProgramFull => "BASIC-MEM-002",
        Error::DivisionByZero => "BASIC-ARITH-001",
        Error::MissingTarget => "BASIC-FLOW-001",
        Error::ForStack => "BASIC-FLOW-002",
        Error::StepZero => "BASIC-FLOW-003",
        Error::Input => "BASIC-INPUT-001",
        Error::Interrupt => "BASIC-RUN-001",
    };
    write_text("ERROR [");
    write_text(code);
    write_char(b']');
    if let Some(line) = line {
        write_text(" line=");
        print_u64(u64::from(line));
    }
    write_text(" col=");
    print_u64(column);
    write_text("\r\n");
}

fn find_line(state: &BasicState, number: u16) -> Option<usize> {
    state
        .lines
        .iter()
        .filter(|line| line.used)
        .position(|line| line.number == number)
}

fn parse_identifier(input: &[u8]) -> Option<(&[u8], usize)> {
    if !input.first()?.is_ascii_alphabetic() {
        return None;
    }
    let mut end = 1;
    while end < input.len() && (input[end].is_ascii_alphanumeric() || input[end] == b'_') {
        end += 1;
    }
    Some((&input[..end], end))
}

fn equals_word(input: &[u8], word: &[u8]) -> bool {
    input.eq_ignore_ascii_case(word)
}
fn starts_word(input: &[u8], word: &[u8]) -> bool {
    input.len() >= word.len()
        && input[..word.len()].eq_ignore_ascii_case(word)
        && input.get(word.len()).is_none_or(|byte| *byte == b' ')
}
fn find_word(input: &[u8], word: &[u8]) -> Option<usize> {
    input
        .windows(word.len())
        .position(|window| window.eq_ignore_ascii_case(word))
}

fn read_line(buffer: &mut [u8]) -> usize {
    let mut length = 0;
    loop {
        let byte = service_read_char();
        if byte == b'\r' || byte == b'\n' {
            write_text("\r\n");
            return length;
        }
        if byte == 8 || byte == 127 {
            if length > 0 {
                length -= 1;
                write_text("\x08 \x08");
            }
            continue;
        }
        if byte.is_ascii() && !byte.is_ascii_control() && length < buffer.len() {
            buffer[length] = byte.to_ascii_uppercase();
            length += 1;
            write_char(byte);
        }
    }
}

fn poll_char() -> Option<u8> {
    let value = unsafe { service_call(SERVICE_POLL_CHAR, 0) };
    (value != 0).then_some(value as u8)
}

fn service_read_char() -> u8 {
    unsafe { service_call(SERVICE_READ_CHAR, 0) as u8 }
}
fn write_char(byte: u8) {
    unsafe {
        service_call(SERVICE_WRITE_CHAR, u64::from(byte));
    }
}
fn write_text(text: &str) {
    for byte in text.bytes() {
        write_char(byte);
    }
}
fn write_bytes(bytes: &[u8]) {
    for byte in bytes {
        write_char(*byte);
    }
}
fn print_u64(mut value: u64) {
    let mut digits = [0u8; 20];
    let mut length = 0;
    if value == 0 {
        write_char(b'0');
        return;
    }
    while value != 0 {
        digits[length] = b'0' + (value % 10) as u8;
        length += 1;
        value /= 10;
    }
    while length != 0 {
        length -= 1;
        write_char(digits[length]);
    }
}

fn print_hex(mut value: u64) {
    let mut digits = [0u8; 16];
    for index in (0..16).rev() {
        let digit = (value & 0xf) as u8;
        digits[index] = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        };
        value >>= 4;
    }
    write_bytes(&digits);
}

fn print_hex_byte(value: u8) {
    let high = value >> 4;
    let low = value & 0xf;
    write_char(if high < 10 {
        b'0' + high
    } else {
        b'a' + high - 10
    });
    write_char(if low < 10 {
        b'0' + low
    } else {
        b'a' + low - 10
    });
}

unsafe fn service_call(number: u64, argument: u64) -> u64 {
    let mut result = argument;
    unsafe {
        asm!("ecall", inlateout("a0") result, in("a7") number, options(nostack));
    }
    result
}
