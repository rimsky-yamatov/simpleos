use std::collections::HashMap;
use std::io::{self, Write};

fn main() {
    println!("SimpleOS emulator — 簡易 BASIC REPL");
    println!("Type HELP for commands. CTRL+D or EXIT to quit.");

    let mut vars: HashMap<String, i64> = HashMap::new();

    loop {
        print!("READY. ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {},
            Err(_) => break,
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        let upper = line.to_uppercase();
        if upper == "EXIT" || upper == "QUIT" {
            break;
        }
        if upper == "HELP" {
            println!("Commands: LET NAME = EXPR   PRINT EXPR   HELP   EXIT");
            continue;
        }

        if upper.starts_with("LET ") {
            if let Some((name, expr)) = parse_let(&line[4..]) {
                match eval_expr(expr, &vars) {
                    Ok(val) => { vars.insert(name.to_string(), val); }
                    Err(e) => println!("Error: {}", e),
                }
            } else {
                println!("Syntax: LET var = expr");
            }
            continue;
        }

        if upper.starts_with("PRINT ") {
            let expr = &line[6..];
            match eval_expr(expr, &vars) {
                Ok(val) => println!("{}", val),
                Err(e) => println!("Error: {}", e),
            }
            continue;
        }

        // 単純に式だけを評価
        match eval_expr(line, &vars) {
            Ok(val) => println!("{}", val),
            Err(e) => println!("Error: {}", e),
        }
    }

    println!("Goodbye.");
}

fn parse_let(s: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 { return None; }
    let name = parts[0].trim();
    let expr = parts[1].trim();
    if name.is_empty() || expr.is_empty() { return None; }
    Some((name, expr))
}

// 非常に簡易な式評価器: + - * / と変数と整数、括弧をサポート
fn eval_expr(s: &str, vars: &HashMap<String, i64>) -> Result<i64, String> {
    let tokens = tokenize(s)?;
    let rpn = shunting_yard(&tokens)?;
    eval_rpn(&rpn, vars)
}

#[derive(Debug, Clone)]
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

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() { chars.next(); continue; }
        if c.is_digit(10) {
            let mut num = 0i64;
            while let Some(&d) = chars.peek() {
                if d.is_digit(10) {
                    num = num * 10 + (d as i64 - '0' as i64);
                    chars.next();
                } else { break; }
            }
            tokens.push(Token::Num(num));
            continue;
        }
        if c.is_alphabetic() {
            let mut name = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    name.push(ch.to_ascii_uppercase());
                    chars.next();
                } else { break; }
            }
            tokens.push(Token::Var(name));
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' => { tokens.push(Token::Op(c)); chars.next(); }
            '(' => { tokens.push(Token::LParen); chars.next(); }
            ')' => { tokens.push(Token::RParen); chars.next(); }
            _ => return Err(format!("Unknown character: {}", c)),
        }
    }
    Ok(tokens)
}

fn precedence(op: char) -> i32 {
    match op {
        '+' | '-' => 1,
        '*' | '/' => 2,
        _ => 0,
    }
}

fn shunting_yard(tokens: &[Token]) -> Result<Vec<Token>, String> {
    let mut output: Vec<Token> = Vec::new();
    let mut ops: Vec<char> = Vec::new();

    for t in tokens {
        match t {
            Token::Num(_) | Token::Var(_) => output.push(t.clone()),
            Token::Op(op) => {
                while let Some(&top) = ops.last() {
                    if precedence(top) >= precedence(*op) {
                        output.push(Token::Op(top));
                        ops.pop();
                    } else { break; }
                }
                ops.push(*op);
            }
            Token::LParen => ops.push('('),
            Token::RParen => {
                while let Some(top) = ops.pop() {
                    if top == '(' { break; }
                    output.push(Token::Op(top));
                }
            }
        }
    }
    while let Some(op) = ops.pop() {
        if op == '(' || op == ')' { return Err("Mismatched parentheses".to_string()); }
        output.push(Token::Op(op));
    }
    Ok(output)
}

fn eval_rpn(tokens: &[Token], vars: &HashMap<String, i64>) -> Result<i64, String> {
    let mut st: Vec<i64> = Vec::new();

    for t in tokens {
        match t {
            Token::Num(n) => st.push(*n),
            Token::Var(name) => {
                if let Some(v) = vars.get(name) { st.push(*v); }
                else { return Err(format!("Undefined variable: {}", name)); }
            }
            Token::Op(op) => {
                if st.len() < 2 { return Err("Not enough operands".to_string()); }
                let b = st.pop().unwrap();
                let a = st.pop().unwrap();
                let res = match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => {
                        if b == 0 { return Err("Division by zero".to_string()); }
                        a / b
                    }
                    _ => return Err(format!("Unknown op {}", op)),
                };
                st.push(res);
            }
            _ => return Err("Invalid token in RPN".to_string()),
        }
    }
    if st.len() != 1 { return Err("Invalid expression".to_string()); }
    Ok(st[0])
}
