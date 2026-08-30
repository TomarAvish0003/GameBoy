use std::io::{self, stdin, stdout, Write};
use gb_core::cpu::Cpu;

pub struct Debugger {
    debugging: bool,
    breakpoints: Vec<u16>,
    read_breakpoints: Vec<u16>,
    write_breakpoints: Vec<u16>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            debugging: false,
            breakpoints: Vec::new(),
            read_breakpoints: Vec::new(),
            write_breakpoints: Vec::new(),
        }
    }

    pub fn is_debugging(&self) -> bool {
        self.debugging
    }

    pub fn print_info(&self) {
        println!("Breakpoint reached!");
    }

    pub fn check_exec_breakpoints(&mut self, pc: u16) {
        if self.breakpoints.contains(&pc) {
            self.debugging = true;
        }
    }

    pub fn check_read_breakpoints(&mut self, addr: u16) {
        if self.read_breakpoints.contains(&addr) {
            self.debugging = true;
        }
    }

    pub fn check_write_breakpoints(&mut self, addr: u16) {
        if self.write_breakpoints.contains(&addr) {
            self.debugging = true;
        }
    }

    pub fn debugloop(&mut self, _gb: &mut Cpu) -> bool {
        loop {
            print!("(gbd) ");
            stdout().flush().unwrap();

            let mut input = String::new();
            let stdin = stdin();
            stdin.read_line(&mut input).expect("unable to parse user input");
            trim_newline(&mut input);

            let words: Vec<&str> = input.split_whitespace().collect();
            if words.is_empty() {
                continue;
            }

            match words[0] {
                "b" => {
                    if words.len() > 1 {
                        let addr = parse_address(words[1]);
                        self.add_breakpoint(addr);
                    } else {
                        println!("Usage: b <hex_address>");
                    }
                }
                "c" => {
                    self.debugging = false;
                    return false;
                }
                "d" => {
                    if words.len() > 1 {
                        let addr = parse_address(words[1]);
                        self.remove_breakpoint(addr);
                    } else {
                        println!("Usage: d <hex_address>");
                    }
                }
                "l" => {
                    self.print_breakpoints();
                }
                "q" => {
                    return true;
                }
                _ => {
                    println!("unknown command");
                }
            }
        }
    }

    fn add_breakpoint(&mut self, bp: Option<u16>) {
        if let Some(addr) = bp {
            if !self.breakpoints.contains(&addr) {
                self.breakpoints.push(addr);
                println!("Breakpoint set at 0x{:04X}", addr);
            }
        }
    }

    fn print_breakpoints(&self) {
        if self.breakpoints.is_empty() {
            println!("There are no set breakpoints");
            return;
        }
        let mut output = "Breakpoints:".to_string();
        for bp in &self.breakpoints {
            output = format!("{} 0x{:04X}", output, bp);
        }
        println!("{}", output);
    }

    fn remove_breakpoint(&mut self, bp: Option<u16>) {
        if let Some(addr) = bp {
            if let Some(pos) = self.breakpoints.iter().position(|&x| x == addr) {
                self.breakpoints.remove(pos);
                println!("Removed breakpoint 0x{:04X}", addr);
            }
        }
    }
}

fn parse_address(input: &str) -> Option<u16> {
    u16::from_str_radix(input.trim_start_matches("0x"), 16).ok()
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}