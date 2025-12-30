use core::fmt::{self, Write};
use log::{Level, LevelFilter, Log, Metadata, Record};
use crate::hal::{console_flush, console_putchar};


struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut i = 0;
        for c in s.chars() {
            console_putchar(c as usize);
            i += 1;
            if i >= 4 {
                console_flush();
                i = 0;
            }
        }
        if i != 0 {
            console_flush();
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?))
    }
}

#[macro_export]
macro_rules! println {
    // 情况 1：只有字符串字面量，没有后续参数
    ($fmt: literal) => {
        $crate::console::print(format_args!(concat!($fmt, "\n")))
    };
    // 情况 2：字符串字面量后面跟着参数
    ($fmt: literal, $($arg: tt)*) => {
        $crate::console::print(format_args!(concat!($fmt, "\n"), $($arg)*))
    };
}

pub fn init() {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
}

struct Logger;
impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        print!("\x1b[{}m", level_to_color_code(record.level()));
        // TODO：实现任务管理模块后再启用下面的代码
        // match current_task() {
        //     Some(task) => println!("pid {}: {}", task.pid.0, record.args()),
        //     None => println!("kernel: {}", record.args()),
        // }
        print!("\x1b[0m")
    }

    fn flush(&self) {}
}

fn level_to_color_code(level: Level) -> u8 {
    match level {
        Level::Error => 31, // Red
        Level::Warn => 93,  // BrightYellow
        Level::Info => 34,  // Blue
        Level::Debug => 32, // Green
        Level::Trace => 90, // BrightBlack
    }
}

