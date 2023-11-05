use core::fmt::{Arguments, Write};
use core::{u8, char, fmt};

use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;

lazy_static! {
    pub static ref PRINT: Mutex<Print> = Mutex::new(Print::new());
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::_print(format_args!($($arg)*))    
    };
}

pub fn _print(args: Arguments) {
    interrupts::without_interrupts(|| {
        PRINT.lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        $crate::print!("{}\n", format_args!($($arg)*));
    }};
}

#[derive(Clone, Copy)]
struct Char {
    charactor: u8,
    color: u8
}

impl Char {
    pub fn new(charactor: char, color: u8) -> Self {
        Self { charactor: charactor as u8, color }
    }

    pub fn empty() -> Self{
        Self { charactor: b' ', color: 0 }
    }
}

const NUM_COL: usize = 80; 
const NUM_ROW: usize = 25;
const BUFFER_SIZE: usize = NUM_COL * NUM_ROW;
const BUFFER_ADRESS: *mut Char = 0xb8000 as *mut Char;

pub struct Print {
    col: i32,
    row: i32,
    color: u8,
    buffer: [Char; BUFFER_SIZE]
}

impl Print {
    pub fn new() -> Self {
        return Self {
            col: 0,
            row: 0,
            color: 0,
            buffer: [Char { charactor: 0, color: 0}; BUFFER_SIZE]
        };
    }

    pub fn set_color(&mut self, foreground: &u8, background: &u8) {
        self.color = foreground + (background << 4);
    }
    
    pub fn clear_row(&mut self, row: i32) {
        for col in 0..NUM_COL {
            self.buffer[col + NUM_COL * row as usize] = Char::empty();
        }
    }

    pub fn print_newline(&mut self) {
        self.col = 0;

        if self.row < (NUM_ROW - 1) as i32 {
            self.row+=1;
            return;
        }

        for row in 1..NUM_ROW {
            for col in 0..NUM_COL {
                let charactor = self.buffer[col + NUM_COL * row];
                self.buffer[col + NUM_COL * (row - 1)] = charactor;
            } 
        }

        self.clear_row((NUM_COL - 1) as i32);
    }

    pub fn print_char(&mut self, charactor: &char) {
        if *charactor == '\n' {
            self.print_newline();
            return;
        }

        if self.col > NUM_COL as i32 {
            self.print_newline();
        }

        self.buffer[self.col as usize + NUM_COL * self.row as usize] = Char::new(*charactor, self.color);

        self.col+=1;
    }

    pub fn print_str(&mut self, string: &str) {
        string.chars().into_iter().for_each(|v| self.print_char(&v));
        self.update();
    }

    fn update(&self) {
        for (i, v) in self.buffer.iter().enumerate() {
            unsafe {
                (*BUFFER_ADRESS.offset(i as isize)).charactor = v.charactor;
                (*BUFFER_ADRESS.offset(i as isize)).color = v.color
            }
        };
    }
}

impl fmt::Write for Print {
    fn write_str(&mut self, s: &str) -> fmt::Result {
       self.print_str(s);
       Ok(())
    }
}
