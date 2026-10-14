use std::collections::{BTreeMap, HashMap};

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

#[derive(Clone, Debug, Default)]
pub struct BasicInterpreter {
    vars: HashMap<String, i64>,
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

    pub fn execute_program(&mut self, program: &Program) -> Result<(), String> {
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
                    let _ = self.eval_expr(expr)?;
                    pc += 1;
                }
                Statement::Input { var } => {
                    let value = self.get_var(var).unwrap_or(0);
                    self.set_var(var, value);
                    pc += 1;
                }
                Statement::Goto { target } => {
                    let idx = lines.iter().position(|(n, _)| *n == *target).ok_or_else(|| format!("Undefined GOTO target {}", target))?;
                    pc = idx;
                }
                Statement::IfThen { cond, then } => {
                    let condition = self.eval_condition(cond)?;
                    if condition {
                        let action = then.trim();
                        if action.to_uppercase().starts_with("GOTO ") {
                            let target = action[5..].trim().parse::<usize>().map_err(|_| format!("Invalid GOTO {}", action))?;
                            let idx = lines.iter().position(|(n, _)| *n == target).ok_or_else(|| format!("Undefined GOTO target {}", target))?;
                            pc = idx;
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
                    pc += 1;
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

fn eval_rpn(tokens: &[Token], vars: &HashMap<String, i64>) -> Result<i64, String> {
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
                        if right == 0 { return Err("Division by zero".to_string()); }
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
