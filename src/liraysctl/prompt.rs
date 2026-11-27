use std::io::{self, Write};

use argon2::{Argon2, password_hash::PasswordHasher};
use chrono::Utc;
use rpassword::prompt_password;

pub(super) fn prompt_and_confirm(prompt: &str) -> Result<String, String> {
    print_prompt(prompt)?;
    let p1 = prompt_password("").map_err(|e| format!("Failed to read password: {e}"))?;
    print_prompt("Confirm password: ")?;
    let p2 =
        prompt_password("").map_err(|e| format!("Failed to read password confirmation: {e}"))?;
    if p1 != p2 {
        return Err("Passwords did not match.".into());
    }
    if p1.trim().is_empty() {
        return Err("Password cannot be empty.".into());
    }
    Ok(p1)
}

pub(super) fn hash_password(password: &str) -> Result<String, String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|_| "Failed to hash password".to_string())
        .map(|ph| ph.to_string())
}

pub(super) fn prompt_token_name() -> Result<String, String> {
    print_prompt("Token name: ")?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("Failed to read token name: {e}"))?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("Token name cannot be empty.".into());
    }
    Ok(trimmed.to_string())
}

pub(super) fn prompt_revoke_target() -> Result<String, String> {
    print_prompt("Token id or name to revoke: ")?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("Failed to read token target: {e}"))?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("Token id or name cannot be empty.".into());
    }
    Ok(trimmed.to_string())
}

pub(super) fn prompt_expiration_seconds() -> Result<i64, String> {
    print_prompt("Expiration (seconds from now). Press Enter for default [30 days]: ")?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("Failed to read input: {e}"))?;
    let trimmed = line.trim();
    let now = Utc::now().timestamp();
    if trimmed.is_empty() {
        return Ok(now + 2_592_000);
    }
    let secs: i64 = trimmed
        .parse()
        .map_err(|e| format!("Invalid number: {e}"))?;
    now.checked_add(secs)
        .ok_or_else(|| format!("Expiration is out of range: {secs} seconds"))
}

pub(super) fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.len());
        }
    }

    fn print_sep(widths: &[usize], left: char, middle: char, right: char) {
        print!("{left}");
        for (i, width) in widths.iter().enumerate() {
            for _ in 0..(*width + 2) {
                print!("─");
            }
            if i + 1 == widths.len() {
                print!("{right}");
            } else {
                print!("{middle}");
            }
        }
        println!();
    }

    print_sep(&widths, '┌', '┬', '┐');
    print!("│");
    for (header, width) in headers.iter().zip(&widths) {
        print!(" {:<width$} │", header, width = *width);
    }
    println!();
    print_sep(&widths, '├', '┼', '┤');

    for row in rows {
        print!("│");
        for (cell, width) in row.iter().zip(&widths) {
            print!(" {:<width$} │", cell, width = *width);
        }
        println!();
    }
    print_sep(&widths, '└', '┴', '┘');
}

fn print_prompt(prompt: &str) -> Result<(), String> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {e}"))
}
