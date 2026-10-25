#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Let { var: String, expr: String },
    Print { expr: String },
    Input { var: String },
    Goto { target: usize },
    IfThen { cond: String, then: String },
    For { var: String, start: String, end: String },
    Next { var: String },
    End,
    Rem(String),
}

pub type Program = BTreeMap<usize, Statement>;

pub trait Console {
    fn read_line(&mut self) -> Option<String>;
    fn write_line(&mut self, s: &str);
}

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

    pub fn clear(&mut self) {
        self.vars.clear();
    }

    pub fn execute_program(&mut self, program: &Program, console: &mut dyn Console) -> Result<(), String> {
        let lines: Vec<(usize, Statement)> = program.iter().map(|(n, stmt)| (*n, stmt.clone())).collect();
        let mut pc = 0usize;

        while pc < lines.len() {
            let (_, stmt) = &lines[pc];
            match stmt {
                Statement::Let { var, expr } => {
                    let value = self.eval_expr(expr)?;
                    self.set_var(var, value);
                    pc += 1;
                }
                Statement::Print { expr } => {
                    let value = self.eval_expr(expr)?;
                    console.write_line(&value.to_string());
                    pc += 1;
                }
                Statement::Input { var } => {
                    if let Some(line) = console.read_line() {
                        if let Ok(v) = line.trim().parse::<i64>() {
                            self.set_var(var, v);
                        } else {
                            return Err("INPUT expects integer".to_string());
                        }
                    } else {
                        return Err("INPUT read failed".to_string());
                    }
                    pc += 1;
                }
                Statement::Goto { target } => {
                    let idx = lines.iter().position(|(n, _)| *n == *target)
                        .ok_or_else(|| format!("Undefined GOTO target {}", target))?;
                    pc = idx;
                }
                Statement::IfThen { cond, then } => {
                    let condition = self.eval_condition(cond)?;
                    if condition {
                        let action = then.trim();
                        if action.to_uppercase().starts_with("GOTO ") {
                            let target = action[5..].trim().parse::<usize>()
                                .map_err(|_| format!("Invalid GOTO {}", action))?;
                            let idx = lines.iter().position(|(n, _)| *n == target)
                                .ok_or_else(|| format!("Undefined GOTO target {}", target))?;
                            pc = idx;
                        } else if action.to_uppercase().starts_with("PRINT ") {
                            let expr = &action[6..];
                            let value = self.eval_expr(expr)?;
                            console.write_line(&value.to_string());
                            pc += 1;
                        } else if action.to_uppercase() == "END" {
                            return Ok(());
                        } else {
                            pc += 1;
                        }
                    } else {
                        pc += 1;
                    }
                }
                Statement::For { var, start, end } => {
                    let start_value = self.eval_expr(start)?;
                    let end_value = self.eval_expr(end)?;
                    self.set_var(var, start_value);
                    if start_value > end_value {
                        let mut skip = pc;
                        while skip < lines.len() {
                            if matches!(lines[skip].1, Statement::Next { .. }) {
                                break;
                            }
                            skip += 1;
                        }
                        pc = skip + 1;
                    } else {
                        pc += 1;
                    }
                }
                Statement::Next { var } => {
                    let current = self.get_var(var).unwrap_or(0);
                    let next_value = current + 1;
                    self.set_var(var, next_value);

                    let mut depth = 0usize;
                    let mut found = None;
                    for i in (0..=pc).rev() {
                        match &lines[i].1 {
                            Statement::Next { .. } => depth += 1,
                            Statement::For { .. } => {
                                if depth == 0 {
                                    found = Some(i);
                                    break;
                                }
                                depth -= 1;
                            }
                            _ => {}
                        }
                    }

                    if let Some(for_idx) = found {
                        if let Statement::For { var: loop_var, end, .. } = &lines[for_idx].1 {
                            let end_value = self.eval_expr(end)?;
                            if next_value <= end_value && loop_var == var {
                                pc = for_idx + 1;
                            } else {
                                pc += 1;
                            }
                        } else {
                            pc += 1;
                        }
                    } else {
                        pc += 1;
                    }
                }
                Statement::End => return Ok(()),
                Statement::Rem(_) => pc += 1,
            }
        }

        Ok(())
    }

    pub fn eval_condition(&self, cond: &str) -> Result<bool, String> {
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

pub fn format_statement(stmt: &Statement) -> String {
    match stmt {
        Statement::Let { var, expr } => format!("LET {} = {}", var, expr),
        Statement::Print { expr } => format!("PRINT {}", expr),
        Statement::Input { var } => format!("INPUT {}", var),
        Statement::Goto { target } => format!("GOTO {}", target),
        Statement::IfThen { cond, then } => format!("IF {} THEN {}", cond, then),
        Statement::For { var, start, end } => format!("FOR {} = {} TO {}", var, start, end),
        Statement::Next { var } => format!("NEXT {}", var),
        Statement::End => "END".to_string(),
        Statement::Rem(msg) => format!("REM {}", msg),
    }
}

pub fn parse_let(s: &str) -> Option<(String, &str)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let name = parts[0].trim();
    let expr = parts[1].trim();
    if name.is_empty() || expr.is_empty() {
        return None;
    }
    Some((name.to_uppercase(), expr))
}

pub fn split_leading_number(input: &str) -> Option<&str> {
    let trimmed = input.trim_start();
    let chars: Vec<char> = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if chars.is_empty() {
        return None;
    }
    Some(trimmed)
}

pub fn parse_program_line(line: &str) -> Option<(usize, Statement)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    while idx < trimmed.len() && trimmed.as_bytes()[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == 0 {
        return None;
    }

    let number = trimmed[..idx].parse::<usize>().ok()?;
    let rest = trimmed[idx..].trim();
    if rest.is_empty() {
        return None;
    }

    let upper = rest.to_uppercase();
    if upper.starts_with("LET ") {
        let rhs = &rest[4..];
        if let Some((name, expr)) = parse_let(rhs) {
            return Some((number, Statement::Let { var: name, expr: expr.to_string() }));
        }
    }
    if upper.starts_with("PRINT ") {
        return Some((number, Statement::Print { expr: rest[6..].trim().to_string() }));
    }
    if upper.starts_with("INPUT ") {
        return Some((number, Statement::Input { var: rest[6..].trim().to_string() }));
    }
    if upper.starts_with("GOTO ") {
        let target = rest[5..].trim().parse::<usize>().ok()?;
        return Some((number, Statement::Goto { target }));
    }
    if upper.starts_with("IF ") {
        let cond_then = &rest[3..];
        let parts: Vec<&str> = cond_then.splitn(2, "THEN").collect();
        if parts.len() == 2 {
            let cond = parts[0].trim();
            let then_body = parts[1].trim();
            return Some((number, Statement::IfThen { cond: cond.to_string(), then: then_body.to_string() }));
        }
    }
    if upper.starts_with("FOR ") {
        let rhs = &rest[4..];
        let mut parts = rhs.splitn(2, "TO");
        let left = parts.next()?.trim();
        let right = parts.next()?.trim();
        let mut decl = left.splitn(2, "=");
        let var = decl.next()?.trim().to_uppercase();
        let start = decl.next()?.trim();
        return Some((number, Statement::For { var, start: start.to_string(), end: right.to_string() }));
    }
    if upper.starts_with("NEXT ") {
        let var = rest[5..].trim().to_uppercase();
        return Some((number, Statement::Next { var }));
    }
    if upper == "END" {
        return Some((number, Statement::End));
    }
    if upper.starts_with("REM ") {
        return Some((number, Statement::Rem(rest[4..].trim().to_string())));
    }

    None
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
                    value = value * 10 + (d as i64 - '0' as i64);
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
            '+' | '-' | '*' | '/' => {
                let unary = match tokens.last() {
                    Some(Token::Num(_)) | Some(Token::Var(_)) | Some(Token::RParen) => false,
                    _ => matches!(ch, '-'),
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
        '*' | '/' => 2,
        '+' | '-' => 1,
        _ => 0,
    }
}

fn shunting_yard(tokens: &[Token]) -> Result<Vec<Token>, String> {
    let mut output = Vec::new();
    let mut ops = Vec::new();

    for token in tokens {
        match token {
            Token::Num(_) | Token::Var(_) => output.push(token.clone()),
            Token::LParen => ops.push('('),
            Token::RParen => {
                while let Some(top) = ops.pop() {
                    if top == '(' {
                        break;
                    }
                    output.push(Token::Op(top));
                }
            }
            Token::Op(op) => {
                while let Some(&top) = ops.last() {
                    if top != '(' && precedence(top) >= precedence(*op) {
                        output.push(Token::Op(ops.pop().unwrap()));
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
    let mut stack = Vec::new();
    for token in tokens {
        match token {
            Token::Num(value) => stack.push(*value),
            Token::Var(name) => {
                let key = name.to_uppercase();
                let value = vars.get(&key).ok_or_else(|| format!("Undefined variable: {}", name))?;
                stack.push(*value);
            }
            Token::Op(op) => {
                if *op == 'u' {
                    let value = stack.pop().ok_or_else(|| "Unary minus missing operand".to_string())?;
                    stack.push(-value);
                    continue;
                }
                let right = stack.pop().ok_or_else(|| "Missing right operand".to_string())?;
                let left = stack.pop().ok_or_else(|| "Missing left operand".to_string())?;
                let value = match *op {
                    '+' => left + right,
                    '-' => left - right,
                    '*' => left * right,
                    '/' => {
                        if right == 0 {
                            return Err("Division by zero".to_string());
                        }
                        left / right
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
    }

    #[test]
    fn parse_if_then() {
        let stmt = parse_program_line("20 IF A > 3 THEN PRINT 42").unwrap();
        assert!(matches!(stmt.1, Statement::IfThen { .. }));
    }
                    }
