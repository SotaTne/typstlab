use std::io::{self, Write};

#[allow(dead_code)]
pub fn print_json(s: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{s}")
}

#[allow(dead_code)]
pub fn print_text(s: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{s}")
}
