use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
enum Statement {
    Let { var: String, expr: String },
    Print { expr: String },
    Input { var: String },
    Goto { target: usize },
    IfThen { cond: String, then: String },
    End,
    Rem(String),
}

#[derive(Clone, Debug)]
struct ProgramLine {
    number: usize,
    statement: Statement,
}

fn main() {
    let fs_dir = ensure_fs_dir();
    println!("SimpleOS emulator v0.2");
    println!("BASIC shell ready. Type HELP, LIST, RUN, SAVE <name>, LOAD <name>, DIR, NEW or EXIT.");
    println!("Filesystem directory: {}", fs_dir.display());

    let mut vars: HashMap<String, i64> = HashMap::new();
    let mut program: BTreeMap<usize, Statement> = BTreeMap::new();

    loop {
        print!("BASIC> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        let upper = line.to_uppercase();
        if upper == "EXIT" || upper == "QUIT" {
            println!("Goodbye.");
            break;
        }

        if upper == "NEW" {
            program.clear();
            vars.clear();
            println!("Program and variables cleared.");
            continue;
        }

        if upper == "HELP" {
            print_help();
            continue;
        }

        if upper == "DIR" || upper == "TREE" {
            list_directory(&fs_dir);
            continue;
        }

        if upper == "LIST" {
            list_program(&program);
            continue;
        }

        if upper == "RUN" {
            match execute_program(&program, &mut vars) {
                Ok(_) => {}
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if let Some(name) = line.strip_prefix("SAVE ") {
            if save_program(&program, &fs_dir, name.trim()) {
                println!("Saved {}", name.trim());
            } else {
                println!("FAILED: save {}", name.trim());
            }
            continue;
        }

        if let Some(name) = line.strip_prefix("LOAD ") {
            match load_program(&fs_dir, name.trim()) {
                Ok(p) => {
                    program = p;
                    println!("Loaded {}", name.trim());
                }
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if let Some(_) = split_leading_number(line) {
            if let Some((number, statement)) = parse_program_line(line) {
                program.insert(number, statement);
                continue;
            }
        }

        if let Some((name, expr)) = parse_let(&line[4..]) {
            match eval_expr(expr, &vars) {
                Ok(v) => {
                    vars.insert(name.to_string(), v);
                    println!("{} = {}", name, v);
                }
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if let Some(expr) = line.strip_prefix("PRINT ") {
            match eval_expr(expr, &vars) {
                Ok(v) => println!("{}", v),
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if let Some(var) = line.strip_prefix("INPUT ") {
            let trimmed = var.trim();
            let mut buf = String::new();
            print!("{}? ", trimmed);
            io::stdout().flush().unwrap();
            match io::stdin().read_line(&mut buf) {
                Ok(_) => {
                    let value = match buf.trim().parse::<i64>() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("ERROR: INPUT expects integer");
                            continue;
                        }
                    };
                    vars.insert(trimmed.to_string(), value);
                }
                Err(_) => println!("ERROR: INPUT read failed"),
            }
            continue;
        }

        match eval_expr(line, &vars) {
            Ok(v) => println!("{}", v),
            Err(e) => println!("ERROR: {}", e),
        }
    }
}

fn ensure_fs_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fs");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn print_help() {
    println!("Commands:");
    println!("  10 PRINT 1 + 2");
    println!("  RUN");
    println!("  LIST");
    println!("  SAVE <basename>");
    println!("  LOAD <basename>");
    println!("  DIR / TREE");
    println!("  NEW");
    println!("  LET A = 10");
    println!("  PRINT A + 5");
    println!("  IF A > 5 THEN PRINT 42");
    println!("  GOTO 20");
    println!("  EXIT");
}

fn list_directory(dir: &Path) {
    let mut entries = fs::read_dir(dir).unwrap().collect::<Vec<_>>();
    entries.sort_by_key(|e| e.as_ref().unwrap().file_name());
    if entries.is_empty() {
        println!("<empty>");
        return;
    }
    for entry in entries {
        let e = entry.unwrap();
        println!("{}", e.file_name().to_string_lossy());
    }
}

fn list_program(program: &BTreeMap<usize, Statement>) {
    if program.is_empty() {
        println!("<empty program>");
        return;
    }
    for (num, stmt) in program.iter() {
        println!("{:>5}  {}", num, format_statement(stmt));
    }
}

fn format_statement(stmt: &Statement) -> String {
    match stmt {
        Statement::Let { var, expr } => format!("LET {} = {}", var, expr),
        Statement::Print { expr } => format!("PRINT {}", expr),
        Statement::Input { var } => format!("INPUT {}", var),
        Statement::Goto { target } => format!("GOTO {}", target),
        Statement::IfThen { cond, then } => format!("IF {} THEN {}", cond, then),
        Statement::End => "END".to_string(),
        Statement::Rem(msg) => format!("REM {}", msg),
    }
}

fn parse_let(s: &str) -> Option<(String, &str)> {
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

fn split_leading_number(input: &str) -> Option<&str> {
    let trimmed = input.trim_start();
    let chars: Vec<char> = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if chars.is_empty() {
        return None;
    }
    Some(trimmed)
}

fn parse_program_line(line: &str) -> Option<(usize, Statement)> {
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
    if upper == "END" {
        return Some((number, Statement::End));
    }
    if upper.starts_with("REM ") {
        return Some((number, Statement::Rem(rest[4..].trim().to_string())));
    }

    None
}

fn save_program(program: &BTreeMap<usize, Statement>, dir: &Path, name: &str) -> bool {
    let raw_name = sanitize_name(name);
    let path = dir.join(format!("{}.bas", raw_name));
    let mut text = String::new();
    for (num, stmt) in program.iter() {
        text.push_str(&format!("{} {}\n", num, format_statement(stmt)));
    }
    fs::write(path, text).is_ok()
}

fn load_program(dir: &Path, name: &str) -> Result<BTreeMap<usize, Statement>, String> {
    let raw_name = sanitize_name(name);
    let path = dir.join(format!("{}.bas", raw_name));
    let content = fs::read_to_string(path).map_err(|_| format!("file {} not found", name))?;
    let mut program = BTreeMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((num, stmt)) = parse_program_line(trimmed) {
            program.insert(num, stmt);
        }
    }
    Ok(program)
}

fn sanitize_name(input: &str) -> String {
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

fn execute_program(program: &BTreeMap<usize, Statement>, vars: &mut HashMap<String, i64>) -> Result<(), String> {
    let lines: Vec<ProgramLine> = program
        .iter()
        .map(|(n, stmt)| ProgramLine { number: *n, statement: stmt.clone() })
        .collect();

    let mut pc = 0usize;

    while pc < lines.len() {
        let line = &lines[pc];
        match &line.statement {
            Statement::Let { var, expr } => {
                let value = eval_expr(expr, vars)?;
                vars.insert(var.clone(), value);
                pc += 1;
            }
            Statement::Print { expr } => {
                let value = eval_expr(expr, vars)?;
                println!("{}", value);
                pc += 1;
            }
            Statement::Input { var } => {
                let mut buf = String::new();
                print!("{}? ", var);
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut buf).map_err(|_| "INPUT failed".to_string())?;
                let value = buf.trim().parse::<i64>().map_err(|_| format!("INPUT requires integer for {}", var))?;
                vars.insert(var.clone(), value);
                pc += 1;
            }
            Statement::Goto { target } => {
                let idx = find_program_index(&lines, *target);
                pc = idx.ok_or_else(|| format!("Undefined GOTO target {}", target))?;
            }
            Statement::IfThen { cond, then } => {
                let ok = eval_condition(cond, vars)?;
                if ok {
                    let action = then.trim();
                    if action.to_uppercase().starts_with("GOTO ") {
                        let target = action[5..].trim().parse::<usize>().map_err(|_| format!("Invalid GOTO {}", action))?;
                        let idx = find_program_index(&lines, target).ok_or_else(|| format!("Undefined GOTO target {}", target))?;
                        pc = idx;
                    } else if action.to_uppercase().starts_with("LET ") {
                        let mut parts = action.splitn(2, '=');
                        let name = parts.next().unwrap_or("").trim().to_uppercase();
                        let expr = parts.next().unwrap_or("").trim();
                        if !name.is_empty() && !expr.is_empty() {
                            let value = eval_expr(expr, vars)?;
                            vars.insert(name, value);
                            pc += 1;
                        } else {
                            pc += 1;
                        }
                    } else if action.to_uppercase().starts_with("PRINT ") {
                        let expr = &action[6..];
                        let value = eval_expr(expr, vars)?;
                        println!("{}", value);
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
            Statement::End => {
                return Ok(());
            }
            Statement::Rem(_) => {
                pc += 1;
            }
        }
    }

    Ok(())
}

fn find_program_index(lines: &[ProgramLine], target: usize) -> Option<usize> {
    lines.iter().position(|line| line.number == target)
}

fn eval_condition(cond: &str, vars: &HashMap<String, i64>) -> Result<bool, String> {
    for op in ["<=", ">=", "<>", "=", "<", ">"] {
        if let Some(pos) = cond.find(op) {
            let left = cond[..pos].trim();
            let right = cond[pos + op.len()..].trim();
            let a = eval_expr(left, vars)?;
            let b = eval_expr(right, vars)?;
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
    Ok(eval_expr(cond, vars)? != 0)
}

#[derive(Debug, Clone)]
enum Token {
    Num(i64),
    Var(String),
    Op(char),
    LParen,
    RParen,
}

fn eval_expr(expr: &str, vars: &HashMap<String, i64>) -> Result<i64, String> {
    let tokens = tokenize(expr)?;
    let postfix = shunting_yard(&tokens)?;
    eval_rpn(&postfix, vars)
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
            let mut value = 0i64;
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
                if ch == '-' {
                    let previous = tokens.last();
                    let unary = match previous {
                        Some(Token::Num(_)) | Some(Token::Var(_)) | Some(Token::RParen) => false,
                        _ => true,
                    };
                    if unary {
                        tokens.push(Token::Op('u'));
                    } else {
                        tokens.push(Token::Op('-'));
                    }
                } else {
                    tokens.push(Token::Op(ch));
                }
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
    let mut operators = Vec::new();

    for token in tokens {
        match token {
            Token::Num(_) | Token::Var(_) => output.push(token.clone()),
            Token::LParen => operators.push('('),
            Token::RParen => {
                while let Some(top) = operators.pop() {
                    if top == '(' {
                        break;
                    }
                    output.push(Token::Op(top));
                }
                if operators.last() == Some(&'(') {
                    operators.pop();
                }
            }
            Token::Op(op) => {
                loop {
                    let top = operators.last().copied();
                    if top.is_some() && top != Some('(') && precedence(top.unwrap()) >= precedence(*op) {
                        output.push(Token::Op(operators.pop().unwrap()));
                    } else {
                        break;
                    }
                }
                operators.push(*op);
            }
        }
    }

    while let Some(op) = operators.pop() {
        if op == '(' {
            return Err("Mismatched parenthesis".to_string());
        }
        output.push(Token::Op(op));
    }

    Ok(output)
}

fn eval_rpn(queue: &[Token], vars: &HashMap<String, i64>) -> Result<i64, String> {
    let mut stack = Vec::new();
    for token in queue {
        match token {
            Token::Num(value) => stack.push(*value),
            Token::Var(name) => {
                let value = vars.get(name).cloned().ok_or_else(|| format!("Undefined variable: {}", name))?;
                stack.push(value);
            }
            Token::Op(op) => {
                if *op == 'u' {
                    let v = stack.pop().ok_or_else(|| "Unary minus missing operand".to_string())?;
                    stack.push(-v);
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
            _ => return Err("Malformed expression".to_string()),
        }
    }

    if stack.len() != 1 {
        return Err("Invalid expression".to_string());
    }

    Ok(stack[0])
}
