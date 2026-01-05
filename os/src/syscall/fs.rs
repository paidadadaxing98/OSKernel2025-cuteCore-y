use crate::fs::{open_dir, open_file, resolve_path, OpenFlags};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_process, current_user_token};

// 已实现
// pub fn sys_getcwd(buf: *const u8, len: usize) -> *const u8 {
//     let token = current_user_token();
//     let process = current_process();
//     let inner = process.inner_exclusive_access();
//     let cwd = &inner.cwd;
//     if cwd.len() + 1 > len {
//         return core::ptr::null();
//     }
//     let mut buffer = UserBuffer::new(translated_byte_buffer(token, buf, len));
//     buffer.write_string(cwd);
//     buf
// }

pub fn sys_getcwd(buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let cwd = &inner.cwd;
    if cwd.len() + 1 > len {
        // return core::ptr::null();
        return -34;
    }
    let mut buffer = UserBuffer::new(translated_byte_buffer(token, buf, len));
    buffer.write_string(cwd);
    buf as isize
}

// 已实现
pub fn sys_chdir(path: *const u8) -> isize {
    let process = current_process();
    let token = current_user_token();
    let path = translated_str(token, path);
    drop(process);
    if open_dir(path.as_str()).is_err() {
        println!("open_dir: {}", path);
        return -1;
    }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    inner.cwd = resolve_path(path.as_str(), inner.cwd.as_str());
    0
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let process = current_process();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = process.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}
// 目前文件可能会因为输入none而发生panic,下面这个版本可以不发生pinic继续执行
// pub fn sys_open(path: *const u8, flags: u32) -> isize {
//     let process = current_process();
//     let token = current_user_token();
//     let path = translated_str(token, path);
//     let flags = match OpenFlags::from_bits(flags) {
//         Some(f) => f,
//         None => return -1,
//     };
//     if let Some(inode) = open_file(path.as_str(), flags) {
//         let mut inner = process.inner_exclusive_access();
//         let fd = inner.alloc_fd();
//         inner.fd_table[fd] = Some(inode);
//         fd as isize
//     } else {
//         -1
//     }
// }
