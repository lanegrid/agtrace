pub trait TerminalWriter: Send {
    fn clear_screen(&mut self);
    fn write_line(&mut self, line: &str);
    fn flush(&mut self);
}

pub struct MockTerminal {
    pub lines: Vec<String>,
    pub clear_count: usize,
    pub flush_count: usize,
}

impl Default for MockTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTerminal {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            clear_count: 0,
            flush_count: 0,
        }
    }
}

impl TerminalWriter for MockTerminal {
    fn clear_screen(&mut self) {
        self.clear_count += 1;
        self.lines.clear();
    }

    fn write_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }

    fn flush(&mut self) {
        self.flush_count += 1;
    }
}

pub struct AnsiTerminal;

impl Default for AnsiTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiTerminal {
    pub fn new() -> Self {
        Self
    }
}

impl TerminalWriter for AnsiTerminal {
    fn clear_screen(&mut self) {
        print!("\x1B[2J\x1B[1;1H");
    }

    fn write_line(&mut self, line: &str) {
        println!("{}", line);
    }

    fn flush(&mut self) {
        use std::io::{self, Write};
        let _ = io::stdout().flush();
    }
}
