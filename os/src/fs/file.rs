use alloc::string::String;
use core::cell::{UnsafeCell};
use crate::fs::inode::OSInode;
use crate::mm::{UserBuffer};

pub trait File: Send + Sync {
    // TODO：先给默认值，后续在改，否则impl File for OSInode的时候会报错
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
    fn get_stat(&self) -> UserStat;
    // 默认返回，在impl File for OSInode里会覆盖
    fn is_dir(&self) -> bool;
    fn get_path(&self) -> String;
}

pub const S_IFREG: u32 = 0o100000; //普通文件
pub const S_IFDIR: u32 = 0o040000; //目录
pub const BLK_SIZE: u32 = 512;



pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad: u64,
    pub st_size: UnsafeCell<i64>,   // 文件大小
    pub st_blksize: u32,
    pub __pad2: i32,
    pub st_blocks: UnsafeCell<u64>, // 占用 512B 块数
}

///由于既需要修改Stat又需要Copy特性所以分成两个了
#[repr(C)]
#[derive(Copy, Clone)]
pub struct UserStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: u32,
    pub st_blocks: u64,
}
