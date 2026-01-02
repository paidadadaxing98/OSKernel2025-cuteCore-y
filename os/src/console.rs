//! 控制台输出与日志模块。
//!
//! 本模块封装了底层 HAL 提供的字符输出接口，
//! 向上提供：
//! - `print!` / `println!` 宏，用于格式化输出
//! - 基于 `log` crate 的日志系统实现
//!
//! # Overview
//! - 字符输出最终通过 HAL 的 `console_putchar` 完成
//! - 输出缓冲按字符数定期调用 `console_flush`
//! - 日志输出支持不同级别，并使用 ANSI 颜色区分
//!
//! # Concurrency Model
//! - 本模块假定运行在内核态
//! - 控制台输出本身不保证并发安全
//! - 调用方需保证在关中断或串行化环境中使用
//!
//! # Safety
//! - 本模块不直接使用 `unsafe`
//! - 但依赖外部保证：
//!   - HAL 层输出接口的正确性
//!   - 日志输出期间不会发生破坏性并发
//!
//! # Invariants
//! - 控制台输出必须保持字符顺序
//! - 日志输出不得引起递归打印或死锁

use crate::hal::{console_flush, console_putchar};
use crate::task::current_task;
use core::fmt::{self, Write};
use log::{Level, LevelFilter, Log, Metadata, Record};

/// 标准输出结构体。
///
/// 该结构体实现 `core::fmt::Write`，
/// 作为格式化输出的最终落地点。
struct Stdout;

impl Write for Stdout {
    /// 将字符串写入控制台。
    ///
    /// 字符逐个通过 `console_putchar` 输出，
    /// 并每输出若干字符后调用 `console_flush`，
    /// 以减少底层 I/O 调用开销。
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

/// 内部打印函数。
///
/// 该函数是 `print!` / `println!` 宏的实际实现，
/// 接收格式化后的参数并输出到控制台。
pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

/// 打印宏（不自动换行）。
///
/// 用法与标准库 `print!` 宏一致。
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?))
    }
}

/// 打印宏（自动追加换行）。
///
/// 用法与标准库 `println!` 宏一致。
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

/// 初始化日志系统。
///
/// 使用 `log` crate 的全局日志接口，
/// 并通过编译期环境变量 `LOG` 设置日志级别。
///
/// 支持的日志级别：
/// - error
/// - warn
/// - info
/// - debug
/// - trace
///
/// 默认关闭日志输出。
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

/// 内核日志记录器。
///
/// 该结构体实现 `log::Log` trait，
/// 是内核中所有日志输出的统一入口。
struct Logger;

impl Log for Logger {
    /// 判断某条日志是否启用。
    ///
    /// 当前实现始终返回 `true`，
    /// 实际过滤逻辑由 `set_max_level` 控制。
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    /// 处理一条日志记录。
    ///
    /// 日志输出格式：
    /// - 根据日志级别设置颜色
    /// - 若存在当前任务，输出任务 ID
    /// - 否则标记为 kernel 日志
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // 设置日志颜色
        print!("\x1b[{}m", level_to_color_code(record.level()));
        match current_task() {
            Some(task) => {
                let tid = task
                    .inner_exclusive_access()
                    .res
                    .as_ref()
                    .map_or(usize::MAX, |res| res.tid);
                println!("pid {}: {}", tid, record.args())
            }
            None => println!("kernel: {}", record.args()),
        }

        // 重置颜色
        print!("\x1b[0m")
    }

    fn flush(&self) {}
}

/// 将日志级别映射为 ANSI 颜色码。
///
/// 用于在控制台中以不同颜色区分日志级别。
fn level_to_color_code(level: Level) -> u8 {
    match level {
        Level::Error => 31, // Red
        Level::Warn => 93,  // BrightYellow
        Level::Info => 34,  // Blue
        Level::Debug => 32, // Green
        Level::Trace => 90, // BrightBlack
    }
}
