//! SimpleOS の BASIC コア。
//!
//! `no_std` + `alloc` で動作し、ホスト(emulator)とカーネルの両方から使われる。
//! 画面入出力は `Console`、プログラム保存先は `Storage` として抽象化している。

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

mod shell;
pub use shell::{Shell, ShellAction, Storage};

pub const VERSION: &str = "SimpleOS 0.3 BASIC";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Let { var: String, expr: String },
    Print { expr: String },
    Input { var: String },
    Goto { target: usize },
    Gosub { target: usize },
    Return,
    IfThen { cond: String, then: String },
    For { var: String, start: String, end: String, step: String },
    Next { var: String },
    Cls,
    End,
    Rem(String),
}

pub type Program = BTreeMap<usize, Statement>;

pub trait Console {
    /// 1行入力する。入力が尽きたら None。
    fn read_line(&mut self) -> Option<String>;
    /// 文字列を出力して改行する。
    fn write_line(&mut self, s: &str);
    /// 改行なしで出力する(INPUT のプロンプトや `PRINT ...;` 用)。
    fn write_prompt(&mut self, _s: &str) {}
    /// 実行中断(Ctrl-C / Esc)が要求されたか。
    fn interrupted(&mut self) -> bool {
        false
    }
    /// 画面を消去する。
    fn clear_screen(&mut self) {}
}

enum Flow {
    Next,
    Jump(usize),
    Stop,
}

const MAX_GOSUB_DEPTH: usize = 1000;

#[derive(Clone, Debug, Default)]
pub struct BasicInterpreter {
    vars: BTreeMap<String, i64>,
}

impl BasicInterpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn eval_expr(&self, expr: &str) -> Result<i64, String> {
        let tokens = tokenize(expr)?;
        let postfix = shunting_yard(&tokens)?;
        eval_rpn(&postfix, &self.vars)
    }

    pub fn set_var(&mut self, name: &str, value: i64) {
        self.vars.insert(name.to_uppercase(), value);
    }

    pub fn get_var(&self, name: &str) -> Option<i64> {
        self.vars.get(&name.to_uppercase()).copied()
    }

    pub fn variables(&self) -> &BTreeMap<String, i64> {
        &self.vars
    }

    pub fn clear(&mut self) {
        self.vars.clear();
    }

    /// 1文だけ即時実行する(ダイレクトモード)。
    pub fn execute_statement(&mut self, stmt: &Statement, console: &mut dyn Console) -> Result<(), String> {
        let lines: Vec<(usize, &Statement)> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        self.exec_stmt(stmt, &lines, 0, &mut stack, console).map(|_| ())
    }

    pub fn execute_program(&mut self, program: &Program, console: &mut dyn Console) -> Result<(), String> {
        let lines: Vec<(usize, &Statement)> = program.iter().map(|(n, s)| (*n, s)).collect();
        let mut stack: Vec<usize> = Vec::new();
        let mut pc = 0usize;
        let mut ticks: u32 = 0;

        while pc < lines.len() {
            ticks = ticks.wrapping_add(1);
            if ticks % 64 == 0 && console.interrupted() {
                return Err(format!("Break in line {}", lines[pc].0));
            }
            let (line_no, stmt) = lines[pc];
            match self.exec_stmt(stmt, &lines, pc, &mut stack, console) {
                Ok(Flow::Next) => pc += 1,
                Ok(Flow::Jump(i)) => pc = i,
                Ok(Flow::Stop) => return Ok(()),
                Err(e) => return Err(format!("{} in line {}", e, line_no)),
            }
        }
        Ok(())
    }

    fn exec_stmt(
        &mut self,
        stmt: &Statement,
        lines: &[(usize, &Statement)],
        pc: usize,
        stack: &mut Vec<usize>,
        console: &mut dyn Console,
    ) -> Result<Flow, String> {
        match stmt {
            Statement::Let { var, expr } => {
                let value = self.eval_expr(expr)?;
                self.set_var(var, value);
                Ok(Flow::Next)
            }
            Statement::Print { expr } => {
                let text = self.format_print(expr)?;
                if expr.trim_end().ends_with(';') {
                    console.write_prompt(&text);
                } else {
                    console.write_line(&text);
                }
                Ok(Flow::Next)
            }
            Statement::Input { var } => {
                console.write_prompt(&format!("{}? ", var));
                let line = console
                    .read_line()
                    .ok_or_else(|| "INPUT read failed".to_string())?;
                let value = line
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| "INPUT expects integer".to_string())?;
                self.set_var(var, value);
                Ok(Flow::Next)
            }
            Statement::Goto { target } => Ok(Flow::Jump(find_line(lines, *target)?)),
            Statement::Gosub { target } => {
                let idx = find_line(lines, *target)?;
                if stack.len() >= MAX_GOSUB_DEPTH {
                    return Err("GOSUB stack overflow".to_string());
                }
                stack.push(pc + 1);
                Ok(Flow::Jump(idx))
            }
            Statement::Return => match stack.pop() {
                Some(idx) => Ok(Flow::Jump(idx)),
                None => Err("RETURN without GOSUB".to_string()),
            },
            Statement::IfThen { cond, then } => {
                if !self.eval_condition(cond)? {
                    return Ok(Flow::Next);
                }
                match parse_statement(then) {
                    Some(inner) => self.exec_stmt(&inner, lines, pc, stack, console),
                    None => Err(format!("Bad THEN clause: {}", then)),
                }
            }
            Statement::For { var, start, end, step } => {
                let s = self.eval_expr(start)?;
                let e = self.eval_expr(end)?;
                let st = self.eval_expr(step)?;
                if st == 0 {
                    return Err("STEP cannot be 0".to_string());
                }
                self.set_var(var, s);
                let skip = if st > 0 { s > e } else { s < e };
                if skip {
                    match find_matching_next(lines, pc) {
                        Some(i) => Ok(Flow::Jump(i + 1)),
                        None => Err("FOR without NEXT".to_string()),
                    }
                } else {
                    Ok(Flow::Next)
                }
            }
            Statement::Next { var } => {
                let for_idx = find_for(lines, pc, var).ok_or_else(|| "NEXT without FOR".to_string())?;
                if let Statement::For { var: loop_var, end, step, .. } = lines[for_idx].1 {
                    let st = self.eval_expr(step)?;
                    let e = self.eval_expr(end)?;
                    let cur = self.get_var(loop_var).unwrap_or(0).wrapping_add(st);
                    self.set_var(loop_var, cur);
                    let cont = if st > 0 { cur <= e } else { cur >= e };
                    if cont {
                        Ok(Flow::Jump(for_idx + 1))
                    } else {
                        Ok(Flow::Next)
                    }
                } else {
                    Ok(Flow::Next)
                }
            }
            Statement::Cls => {
                console.clear_screen();
                Ok(Flow::Next)
            }
            Statement::End => Ok(Flow::Stop),
            Statement::Rem(_) => Ok(Flow::Next),
        }
    }

    fn format_print(&self, expr: &str) -> Result<String, String> {
        let mut out = String::new();
        let mut item = String::new();
        let mut in_str = false;
        for ch in expr.chars() {
            if ch == '"' {
                in_str = !in_str;
                item.push(ch);
            } else if !in_str && (ch == ';' || ch == ',') {
                self.append_print_item(&mut out, &item)?;
                item.clear();
                if ch == ',' {
                    out.push(' ');
                }
            } else {
                item.push(ch);
            }
        }
        if in_str {
            return Err("Unterminated string".to_string());
        }
        self.append_print_item(&mut out, &item)?;
        Ok(out)
    }

    fn append_print_item(&self, out: &mut String, item: &str) -> Result<(), String> {
        let t = item.trim();
        if t.is_empty() {
            return Ok(());
        }
        if t.starts_with('"') {
            if t.len() < 2 || !t.ends_with('"') {
                return Err("Syntax error in PRINT".to_string());
            }
            out.push_str(&t[1..t.len() - 1]);
        } else {
            let v = self.eval_expr(t)?;
            out.push_str(&v.to_string());
        }
        Ok(())
    }

    pub fn eval_condition(&self, cond: &str) -> Result<bool, String> {
        let upper = cond.to_ascii_uppercase();
        if let Some(pos) = upper.find(" OR ") {
            return Ok(self.eval_condition(&cond[..pos])? || self.eval_condition(&cond[pos + 4..])?);
        }
        if let Some(pos) = upper.find(" AND ") {
            return Ok(self.eval_condition(&cond[..pos])? && self.eval_condition(&cond[pos + 5..])?);
        }
        for op in ["<=", ">=", "<>", "=", "<", ">"] {
            if let Some(pos) = cond.find(op) {
                let left = cond[..pos].trim();
                let right = cond[pos + op.len()..].trim();
                let a = self.eval_expr(left)?;
                let b = self.eval_expr(right)?;
                let result = match op {
                    "<=" => a <= b,
                    ">=" => a >= b,
                    "<>" => a != b,
                    "=" => a == b,
                    "<" => a < b,
                    ">" => a > b,
                    _ => false,
                };
                return Ok(result);
            }
        }
        Ok(self.eval_expr(cond)? != 0)
    }
}

fn find_line(lines: &[(usize, &Statement)], target: usize) -> Result<usize, String> {
    lines
        .iter()
        .position(|&(n, _)| n == target)
        .ok_or_else(|| format!("Undefined line {}", target))
}

/// pc にある FOR に対応する NEXT の位置を探す。
fn find_matching_next(lines: &[(usize, &Statement)], pc: usize) -> Option<usize> {
    let mut depth = 0usize;
    for i in (pc + 1)..lines.len() {
        match lines[i].1 {
            Statement::For { .. } => depth += 1,
            Statement::Next { .. } => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// pc にある NEXT に対応する FOR の位置を探す。
fn find_for(lines: &[(usize, &Statement)], pc: usize, var: &str) -> Option<usize> {
    let mut depth = 0usize;
    for i in (0..pc).rev() {
        match lines[i].1 {
            Statement::Next { .. } => depth += 1,
            Statement::For { var: v, .. } => {
                if var.is_empty() {
                    if depth == 0 {
                        return Some(i);
                    }
                    depth -= 1;
                } else if v == var {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn format_statement(stmt: &Statement) -> String {
    match stmt {
        Statement::Let { var, expr } => format!("LET {} = {}", var, expr),
        Statement::Print { expr } => {
            if expr.is_empty() {
                "PRINT".to_string()
            } else {
                format!("PRINT {}", expr)
            }
        }
        Statement::Input { var } => format!("INPUT {}", var),
        Statement::Goto { target } => format!("GOTO {}", target),
        Statement::Gosub { target } => format!("GOSUB {}", target),
        Statement::Return => "RETURN".to_string(),
        Statement::IfThen { cond, then } => format!("IF {} THEN {}", cond, then),
        Statement::For { var, start, end, step } => {
            if step.trim() == "1" {
                format!("FOR {} = {} TO {}", var, start, end)
            } else {
                format!("FOR {} = {} TO {} STEP {}", var, start, end, step)
            }
        }
        Statement::Next { var } => {
            if var.is_empty() {
                "NEXT".to_string()
            } else {
                format!("NEXT {}", var)
            }
        }
        Statement::Cls => "CLS".to_string(),
        Statement::End => "END".to_string(),
        Statement::Rem(msg) => {
            if msg.is_empty() {
                "REM".to_string()
            } else {
                format!("REM {}", msg)
            }
        }
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn parse_let(s: &str) -> Option<(String, &str)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let name = parts[0].trim();
    let expr = parts[1].trim();
    if name.is_empty() || expr.is_empty() || !is_identifier(name) {
        return None;
    }
    Some((name.to_uppercase(), expr))
}

pub fn split_leading_number(input: &str) -> Option<&str> {
    let trimmed = input.trim_start();
    if trimmed.bytes().next().map_or(false, |b| b.is_ascii_digit()) {
        Some(trimmed)
    } else {
        None
    }
}

/// 大文字小文字を無視してキーワードを取り除く。キーワードの後は空白/終端/引用符/括弧のみ許す。
fn strip_keyword<'a>(text: &'a str, kw: &str) -> Option<&'a str> {
    let n = kw.len();
    if !text.is_char_boundary(n) {
        return None;
    }
    if !text[..n].eq_ignore_ascii_case(kw) {
        return None;
    }
    let rest = &text[n..];
    if rest.is_empty() {
        return Some("");
    }
    if rest.starts_with(char::is_whitespace) || rest.starts_with('"') || rest.starts_with('(') {
        return Some(rest.trim());
    }
    None
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// 単語単位でキーワード(大文字化済みの文字列内)を探す。
fn find_word(upper: &str, word: &str) -> Option<usize> {
    let bytes = upper.as_bytes();
    for (i, _) in upper.match_indices(word) {
        let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
        let end = i + word.len();
        let after_ok = end >= bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return Some(i);
        }
    }
    None
}

/// 行番号なしの1文をパースする。
pub fn parse_statement(text: &str) -> Option<Statement> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }

    if let Some(rest) = strip_keyword(t, "REM") {
        return Some(Statement::Rem(rest.to_string()));
    }
    if let Some(rest) = strip_keyword(t, "PRINT") {
        return Some(Statement::Print { expr: rest.to_string() });
    }
    if let Some(rest) = strip_keyword(t, "LET") {
        let (var, expr) = parse_let(rest)?;
        return Some(Statement::Let { var, expr: expr.to_string() });
    }
    if let Some(rest) = strip_keyword(t, "INPUT") {
        if !is_identifier(rest) {
            return None;
        }
        return Some(Statement::Input { var: rest.to_uppercase() });
    }
    if let Some(rest) = strip_keyword(t, "GOTO") {
        let target = rest.parse::<usize>().ok()?;
        return Some(Statement::Goto { target });
    }
    if let Some(rest) = strip_keyword(t, "GOSUB") {
        let target = rest.parse::<usize>().ok()?;
        return Some(Statement::Gosub { target });
    }
    if let Some(rest) = strip_keyword(t, "RETURN") {
        if rest.is_empty() {
            return Some(Statement::Return);
        }
        return None;
    }
    if let Some(rest) = strip_keyword(t, "END") {
        if rest.is_empty() {
            return Some(Statement::End);
        }
        return None;
    }
    if let Some(rest) = strip_keyword(t, "CLS") {
        if rest.is_empty() {
            return Some(Statement::Cls);
        }
        return None;
    }
    if let Some(rest) = strip_keyword(t, "IF") {
        let upper = rest.to_ascii_uppercase();
        let pos = find_word(&upper, "THEN")?;
        let cond = rest[..pos].trim();
        let then_body = rest[pos + 4..].trim();
        if cond.is_empty() || then_body.is_empty() {
            return None;
        }
        // THEN 100  は GOTO 100 の省略形
        if then_body.bytes().all(|b| b.is_ascii_digit()) {
            return Some(Statement::IfThen {
                cond: cond.to_string(),
                then: format!("GOTO {}", then_body),
            });
        }
        return Some(Statement::IfThen { cond: cond.to_string(), then: then_body.to_string() });
    }
    if let Some(rest) = strip_keyword(t, "FOR") {
        let upper = rest.to_ascii_uppercase();
        let to_pos = find_word(&upper, "TO")?;
        let (var, start) = parse_let(rest[..to_pos].trim())?;
        let after = &rest[to_pos + 2..];
        let after_upper = &upper[to_pos + 2..];
        let (end, step) = match find_word(after_upper, "STEP") {
            Some(sp) => (after[..sp].trim(), after[sp + 4..].trim()),
            None => (after.trim(), "1"),
        };
        if end.is_empty() || step.is_empty() {
            return None;
        }
        return Some(Statement::For {
            var,
            start: start.to_string(),
            end: end.to_string(),
            step: step.to_string(),
        });
    }
    if let Some(rest) = strip_keyword(t, "NEXT") {
        if !rest.is_empty() && !is_identifier(rest) {
            return None;
        }
        return Some(Statement::Next { var: rest.to_uppercase() });
    }

    // `A = 5` のような LET 省略形
    if let Some((var, expr)) = parse_let(t) {
        return Some(Statement::Let { var, expr: expr.to_string() });
    }

    None
}

pub fn parse_program_line(line: &str) -> Option<(usize, Statement)> {
    let trimmed = line.trim();
    let digits = trimmed.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let number = trimmed[..digits].parse::<usize>().ok()?;
    let stmt = parse_statement(&trimmed[digits..])?;
    Some((number, stmt))
}

/// プログラムを保存用テキストに変換する。
pub fn program_to_text(program: &Program) -> String {
    let mut text = String::new();
    for (num, stmt) in program.iter() {
        text.push_str(&format!("{} {}\n", num, format_statement(stmt)));
    }
    text
}

/// 保存用テキストからプログラムを復元する(読めない行は無視)。
pub fn program_from_text(text: &str) -> Program {
    let mut program = BTreeMap::new();
    for line in text.lines() {
        if let Some((num, stmt)) = parse_program_line(line) {
            program.insert(num, stmt);
        }
    }
    program
}

pub fn sanitize_name(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .collect();
    if cleaned.is_empty() {
        "program".to_string()
    } else {
        cleaned
    }
}

#[derive(Clone, Debug)]
enum Token {
    Num(i64),
    Var(String),
    Op(char),
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let mut chars = s.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch.is_ascii_digit() {
            let mut value = 0_i64;
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    value = value.wrapping_mul(10).wrapping_add(d as i64 - '0' as i64);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Num(value));
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut ident = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    ident.push(c.to_ascii_uppercase());
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Var(ident));
            continue;
        }
        match ch {
            '+' | '-' | '*' | '/' | '%' => {
                let unary = match tokens.last() {
                    Some(Token::Num(_)) | Some(Token::Var(_)) | Some(Token::RParen) => false,
                    _ => ch == '-',
                };
                tokens.push(Token::Op(if unary { 'u' } else { ch }));
                chars.next();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            _ => return Err(format!("Unknown character: '{}'", ch)),
        }
    }

    Ok(tokens)
}

fn precedence(op: char) -> i32 {
    match op {
        'u' => 3,
        '*' | '/' | '%' => 2,
        '+' | '-' => 1,
        _ => 0,
    }
}

fn shunting_yard(tokens: &[Token]) -> Result<Vec<Token>, String> {
    let mut output = Vec::new();
    let mut ops: Vec<char> = Vec::new();

    for token in tokens {
        match token {
            Token::Num(_) | Token::Var(_) => output.push(token.clone()),
            Token::LParen => ops.push('('),
            Token::RParen => {
                let mut matched = false;
                while let Some(top) = ops.pop() {
                    if top == '(' {
                        matched = true;
                        break;
                    }
                    output.push(Token::Op(top));
                }
                if !matched {
                    return Err("Mismatched parenthesis".to_string());
                }
            }
            Token::Op(op) => {
                while let Some(&top) = ops.last() {
                    if top != '(' && precedence(top) >= precedence(*op) {
                        output.push(Token::Op(top));
                        ops.pop();
                    } else {
                        break;
                    }
                }
                ops.push(*op);
            }
        }
    }

    while let Some(op) = ops.pop() {
        if op == '(' {
            return Err("Mismatched parenthesis".to_string());
        }
        output.push(Token::Op(op));
    }

    Ok(output)
}

fn eval_rpn(tokens: &[Token], vars: &BTreeMap<String, i64>) -> Result<i64, String> {
    let mut stack: Vec<i64> = Vec::new();
    for token in tokens {
        match token {
            Token::Num(value) => stack.push(*value),
            Token::Var(name) => {
                let value = vars
                    .get(name)
                    .ok_or_else(|| format!("Undefined variable: {}", name))?;
                stack.push(*value);
            }
            Token::Op(op) => {
                if *op == 'u' {
                    let value = stack.pop().ok_or_else(|| "Unary minus missing operand".to_string())?;
                    stack.push(value.wrapping_neg());
                    continue;
                }
                let right = stack.pop().ok_or_else(|| "Missing right operand".to_string())?;
                let left = stack.pop().ok_or_else(|| "Missing left operand".to_string())?;
                let value = match *op {
                    '+' => left.wrapping_add(right),
                    '-' => left.wrapping_sub(right),
                    '*' => left.wrapping_mul(right),
                    '/' => {
                        if right == 0 {
                            return Err("Division by zero".to_string());
                        }
                        left.wrapping_div(right)
                    }
                    '%' => {
                        if right == 0 {
                            return Err("Division by zero".to_string());
                        }
                        left.wrapping_rem(right)
                    }
                    _ => return Err(format!("Unsupported operator: {}", op)),
                };
                stack.push(value);
            }
            Token::LParen | Token::RParen => return Err("Malformed expression".to_string()),
        }
    }

    if stack.len() != 1 {
        return Err("Invalid expression".to_string());
    }

    Ok(stack[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cap {
        out: Vec<String>,
        input: Vec<String>,
    }

    impl Console for Cap {
        fn read_line(&mut self) -> Option<String> {
            if self.input.is_empty() {
                None
            } else {
                Some(self.input.remove(0))
            }
        }
        fn write_line(&mut self, s: &str) {
            self.out.push(s.to_string());
        }
        fn write_prompt(&mut self, s: &str) {
            self.out.push(s.to_string());
        }
    }

    fn run_with(src: &str, input: &[&str]) -> Vec<String> {
        let program = program_from_text(src);
        let mut interp = BasicInterpreter::new();
        let mut cap = Cap {
            out: Vec::new(),
            input: input.iter().map(|s| s.to_string()).collect(),
        };
        interp.execute_program(&program, &mut cap).unwrap();
        cap.out
    }

    fn run(src: &str) -> Vec<String> {
        run_with(src, &[])
    }

    #[test]
    fn parse_basic_line() {
        let stmt = parse_program_line("10 PRINT A + 1").unwrap();
        assert_eq!(stmt.0, 10);
        assert!(matches!(stmt.1, Statement::Print { .. }));
    }

    #[test]
    fn evaluate_expression() {
        let mut interp = BasicInterpreter::new();
        interp.set_var("A", 5);
        assert_eq!(interp.eval_expr("A + 2").unwrap(), 7);
        assert_eq!(interp.eval_expr("2 + 3 * 4").unwrap(), 14);
        assert_eq!(interp.eval_expr("(2 + 3) * 4").unwrap(), 20);
        assert_eq!(interp.eval_expr("-5 + 10 % 3").unwrap(), -4);
        assert_eq!(interp.eval_expr("7 / 2").unwrap(), 3);
        assert!(interp.eval_expr("(1 + 2").is_err());
        assert!(interp.eval_expr("1 + 2)").is_err());
        assert!(interp.eval_expr("1 / 0").is_err());
    }

    #[test]
    fn parse_if_then() {
        let stmt = parse_program_line("20 IF A > 3 THEN PRINT 42").unwrap();
        assert!(matches!(stmt.1, Statement::IfThen { .. }));
    }

    #[test]
    fn format_round_trip() {
        let (_, stmt) = parse_program_line("10 FOR I = 1 TO 5 STEP 2").unwrap();
        assert_eq!(format_statement(&stmt), "FOR I = 1 TO 5 STEP 2");
        let (_, stmt) = parse_program_line("10 for i = 1 to 5").unwrap();
        assert_eq!(format_statement(&stmt), "FOR I = 1 TO 5");
    }

    #[test]
    fn demo_program_runs() {
        let out = run(
            "10 REM demo\n20 LET A = 10\n30 PRINT \"A = \"; A\n40 PRINT \"A * 3 = \"; A * 3\n\
             50 IF A > 5 THEN PRINT \"OK\"\n60 FOR I = 1 TO 3\n70 PRINT \"LOOP \"; I\n80 NEXT I\n90 END\n",
        );
        assert_eq!(out, ["A = 10", "A * 3 = 30", "OK", "LOOP 1", "LOOP 2", "LOOP 3"]);
    }

    #[test]
    fn for_with_negative_step() {
        let out = run("10 FOR I = 10 TO 1 STEP -3\n20 PRINT I\n30 NEXT I\n");
        assert_eq!(out, ["10", "7", "4", "1"]);
    }

    #[test]
    fn gosub_and_return() {
        let out = run("10 GOSUB 100\n20 PRINT \"back\"\n30 END\n100 PRINT \"sub\"\n110 RETURN\n");
        assert_eq!(out, ["sub", "back"]);
    }

    #[test]
    fn nested_loops_primes() {
        let out = run(
            "10 FOR N = 2 TO 30\n20 LET P = 1\n30 FOR D = 2 TO N - 1\n40 IF N % D = 0 THEN LET P = 0\n\
             50 NEXT D\n60 IF P = 1 THEN PRINT N\n70 NEXT N\n80 END\n",
        );
        assert_eq!(out, ["2", "3", "5", "7", "11", "13", "17", "19", "23", "29"]);
    }

    #[test]
    fn logical_conditions_and_goto() {
        let out = run(
            "10 IF 1 = 1 AND 2 < 3 THEN PRINT \"y\"\n20 IF 1 = 2 OR 3 = 3 THEN 40\n30 PRINT \"skipped\"\n40 PRINT \"end\"\n",
        );
        assert_eq!(out, ["y", "end"]);
    }

    #[test]
    fn input_reads_value() {
        let out = run_with("10 INPUT N\n20 PRINT N * N\n", &["12"]);
        assert_eq!(out, ["N? ", "144"]);
    }

    #[test]
    fn print_semicolon_suppresses_newline() {
        let out = run("10 PRINT \"a\";\n20 PRINT \"b\"\n");
        assert_eq!(out, ["a", "b"]);
    }

    #[test]
    fn program_text_round_trip() {
        let program = program_from_text("10 PRINT \"hi\"\n20 GOTO 10\n");
        let text = program_to_text(&program);
        assert_eq!(text, "10 PRINT \"hi\"\n20 GOTO 10\n");
    }
}
