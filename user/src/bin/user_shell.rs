#![no_std]
#![no_main]

extern crate alloc;
extern crate user;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use user::console::getchar;
use user::{close, dup, exec, fork, open, pipe, println, print, waitpid, OpenFlags};

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

/// 存储单个进程的参数和重定向信息
struct Command {
    args_copy: Vec<String>, // 拥有所有权，防止悬垂指针
    args_addr: Vec<*const u8>,  // 传递给内核的指针数组
    input: String,
    output: String,
}

impl Command {
    pub fn new(cmd_str: &str) -> Self {
        let mut args: Vec<String> = cmd_str
            .split_whitespace()
            .map(|arg| format!("{}\0", arg))
            .collect();

        let mut input = String::new();
        let output = String::new();

        // 提取并移除输入重定向 <
        if let Some(i) = args.iter().position(|arg| arg == "<\0") {
            if i + 1 < args.len() {
                input = args[i + 1].clone();
                args.drain(i..=i + 1);
            }
        }

        // 提取并移除输出重定向 >
        let mut args_addr: Vec<*const u8> = args.iter().map(|arg| arg.as_ptr()).collect();
        args_addr.push(core::ptr::null());

        Self {
            args_copy: args,
            args_addr,
            input,
            output,
        }
    }
}

/// 将文件描述符重定向到标准输入/输出
fn redirect(old_fd: usize, new_fd: usize) {
    close(new_fd);
    dup(old_fd);
    close(old_fd);
}

/// 处理文件重定向逻辑
fn handle_file_redirection(input: &str, output: &str) {
    if !input.is_empty() {
        // 去掉末尾的 \0 进行 open 调用
        let path = &input[..input.len() - 1];
        let fd = open(path, OpenFlags::RDONLY);
        if fd != -1 {
            redirect(fd as usize, 0);
        }
    }
    if !output.is_empty() {
        let path = &output[..output.len() - 1];
        let fd = open(path, OpenFlags::CREATE | OpenFlags::WRONLY);
        if fd != -1 {
            redirect(fd as usize, 1);
        }
    }
}


#[no_mangle]
fn main() -> i32{
    println!("Rust Shell Initialized.");
    let mut line = String::new();

    loop {
        print!(">> ");
        line.clear();

        loop {
            let c = getchar();
            match c {
               CR | LF => {
                   println!(" ");
                   break;
               }
                BS | DL => {
                    if !line.is_empty() {
                        print!("{}", BS as char);
                        print!(" ");
                        print!("{}", BS as char);
                        line.pop();
                    }
                }
                _ => {
                    print!("{}", c as char);
                    line.push(c as char);
                }
            }
        }

        if line.is_empty() { continue; }

        // 解析管道命令
        let cmd_parts: Vec<&str> = line.split('|').collect();
        let commands: Vec<Command> = cmd_parts.iter().map(|&s| Command::new(s)).collect();
        let num_cmds = commands.len();

        // 创建 N-1 个管道
        let pipes: Vec<[usize; 2]> = (0..num_cmds.saturating_sub(1))
            .map(|_| {
                let mut fd = [0usize; 2];
                pipe(&mut fd);
                fd
            })
            .collect();

        let mut children = Vec::new();



        for (i, cmd) in commands.iter().enumerate() {
            let pid = fork();
            if pid == 0 {
                // 子进程逻辑

                // 1. 处理管道连接
                if i > 0 {
                    // 不是第一个命令，从上一个管道读
                    redirect(pipes[i - 1][0], 0);
                }
                if i < num_cmds - 1 {
                    // 不是最后一个命令，向当前管道写
                    redirect(pipes[i][1], 1);
                }

                // 2. 处理文件重定向
                handle_file_redirection(&cmd.input, &cmd.output);

                // 3. 必须关闭子进程继承的所有管道 FD，否则读端会阻塞
                for p in &pipes {
                    close(p[0]);
                    close(p[1]);
                }

                // 4. 执行
                if exec(cmd.args_copy[0].as_str(), cmd.args_addr.as_slice()) == -1 {
                    println!("Exec failed: {}", cmd.args_copy[0]);
                }
                unreachable!();
            } else {
                children.push(pid);
            }
        }

        // 父进程清理：关闭所有管道 FD
        for p in pipes {
            close(p[0]);
            close(p[1]);
        }

        // 等待所有子进程
        for pid in children {
            let mut status = 0i32;
            waitpid(pid as usize, &mut status);
        }
    }
}