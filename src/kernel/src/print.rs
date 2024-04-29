use core::fmt;
use core::fmt::{Arguments, Write};

use crate::BootInformation;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use x86_64::instructions::interrupts;

use self::ttf_renderer::TtfRenderer;

pub mod ttf_parser;
pub mod ttf_renderer;

pub static DRIVER: OnceCell<Mutex<Print>> = OnceCell::uninit();

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::_print(format_args!($($arg)*))
    };
}

pub fn _print(args: Arguments) {
    interrupts::without_interrupts(|| {
        if let Some(driver) = DRIVER.get() {
            driver.lock().write_fmt(args).unwrap();
        } else {
            panic!("Use of uninitialize driver (Print driver)");
        }
    });
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        $crate::print!("{}\n", format_args!($($arg)*));
    }};
}

pub fn init(bootinfo: &mut BootInformation, foreground_color: u32) {
    DRIVER.init_once(|| Mutex::new(Print::new(bootinfo, foreground_color)));
}

pub struct Print {
    renderer: TtfRenderer,
}

impl Print {
    pub fn new(bootinfo: &mut BootInformation, foreground: u32) -> Self {
        return Self {
            renderer: TtfRenderer::new(bootinfo, foreground),
        };
    }

    pub fn set_color(&mut self, foreground: &u32) {
        self.renderer.set_color(foreground);
    }

    pub fn print_str(&mut self, string: &str) {
        self.renderer.put_str(string);
    }
}

impl fmt::Write for Print {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_str(s);
        Ok(())
    }
}
