use crate::console::print;
use crate::fs::fat32::FAT_FS;
use crate::fs::FatFsBlockDevice;
use crate::mm::UserBuffer;
use crate::sync::UPIntrFreeCell;
use crate::task::current_process;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use fatfs::{DefaultTimeProvider, Dir, File, FileSystem, LossyOemCpConverter, Read, Write};
use lazy_static::lazy_static;

pub struct OSInode {
    readable: bool,
    writable: bool,
    // 未来如果需要支持多核，则需要改用更强的同步机制（如 spin::Mutex）。
    file: UPIntrFreeCell<FatType>,
}

pub enum FatType {
    File(File<'static, FatFsBlockDevice, DefaultTimeProvider, LossyOemCpConverter>),
    Dir(Dir<'static, FatFsBlockDevice, DefaultTimeProvider, LossyOemCpConverter>),
}

// 理由：在单核环境下，UPIntrFreeCell 通过屏蔽中断保证了原子性。
// 即使 fatfs::File 本身不是 Send/Sync，但由于保证了同一时间
// 只有一个内核任务能通过该 Cell 访问它，所以可以安全地在任务间转移它。
// 单核 + 中断屏蔽 + 同一时间只有一个任务访问
unsafe impl Send for OSInode {}
unsafe impl Sync for OSInode {}

impl OSInode {
    pub fn new(readable: bool, writable: bool, file: FatType) -> Self {
        Self {
            readable,
            writable,
            file: unsafe { UPIntrFreeCell::new(file) },
        }
    }

    /// 当前 read_all 时从 offset 到 EOF 而不是从文件开始到 EOF
    /// 把注释部分取消则从文件开始到 EOF
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.file.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        match &mut *inner {
            FatType::File(file) => {
                // file.seek(SeekFrom::Start(0)).unwrap();
                loop {
                    let len = file.read(&mut buffer);
                    let size = len.unwrap();
                    if size == 0 {
                        break;
                    }
                    v.extend_from_slice(&buffer[..size]);
                }
            }
            FatType::Dir(_) => {
                log::debug!("Get a Dir to read, which is not supported");
            }
        }
        v
    }
}
lazy_static! {
    pub static ref ROOT_DIR: UPIntrFreeCell<Dir<'static, FatFsBlockDevice, DefaultTimeProvider, LossyOemCpConverter>> = {
        // 获取文件系统的锁
        let fs_guard = FAT_FS.lock();
        // 关键点：fatfs 的 root_dir() 会借用 FileSystem。
        // 在 static 初始化块中，需要确保引用的合法性。
        let fs_static: &'static FileSystem<FatFsBlockDevice, DefaultTimeProvider, LossyOemCpConverter> =
            unsafe { &*(fs_guard.deref() as *const _) };

        let root_dir = fs_static.root_dir();
        unsafe {
            UPIntrFreeCell::new(root_dir)
        }
    };
}

pub fn list_apps() {
    println!("List of applications:");
    for entry in ROOT_DIR.exclusive_access().iter() {
        let entry = entry.expect("Failed to read directory entry");
        let file_name = entry.file_name();
        let attributes = if entry.is_dir() { "DIR" } else { "FILE" };
        let size = entry.len();
        println!(
            "[[{}]], FileName: {}, Size: {}",
            attributes, file_name, size
        );
    }
}

bitflags! {
    pub struct OpenFlags: u32 {
        // 只读
        const RDONLY = 0;
        // 只写
        const WRONLY = 1 << 0;
        // 读写
        const RDWR = 1 << 1;
        // 创建
        const CREATE = 1 << 9;
        // 截断（若存在则以可写方式打开，但是长度清空为0）
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    pub fn read_write(&self) -> (bool, bool) {
        if self.contains(Self::WRONLY) {
            (false, true)
        } else if self.contains(Self::RDWR) {
            (true, true)
        } else {
            (true, false)
        }
    }
}

impl super::File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.file.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            match &mut *inner {
                FatType::File(file) => {
                    let read_size = file.read(slice).unwrap();
                    total_read_size += read_size;
                    if read_size < slice.len() {
                        break;
                    }
                }
                FatType::Dir(_) => {
                    log::debug!("Get a Dir to read, which is not supported");
                }
            }
        }
        total_read_size
    }

    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.file.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            match &mut *inner {
                FatType::File(file) => {
                    let write_size = file.write(slice).unwrap();
                    total_write_size += write_size;
                    if write_size < slice.len() {
                        break;
                    }
                }
                FatType::Dir(_) => {
                    log::debug!("Get a Dir to write, which is not supported");
                }
            }
        }
        total_write_size
    }
}

pub fn resolve_path(relative: &str, base: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();

    let is_absolute = relative.starts_with("/");

    if !is_absolute {
        for component in base.split("/") {
            match component {
                "" | "." => continue,
                ".." => {
                    if !stack.is_empty() {
                        stack.pop();
                    }
                }
                _ => stack.push(component),
            }
        }
    }

    for component in relative.split("/") {
        match component {
            "" | "." => continue,
            ".." => {
                if !stack.is_empty() {
                    stack.pop();
                }
            }
            _ => stack.push(component),
        }
    }

    let mut result = String::from("/");
    result.push_str(&stack.join("/"));
    result
}

pub fn open_initproc(flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    let root_dir = ROOT_DIR.exclusive_access();
    root_dir
        .open_file("initproc")
        .ok()
        .map(|inode| Arc::new(OSInode::new(readable, writable, FatType::File(inode))))
}

// 实现不完整，还未支持文件的所有权描述
pub fn open_file(path: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();

    let full_path = {
        let proc = current_process();
        let inner = proc.inner_exclusive_access();
        let cwd = &inner.cwd;
        resolve_path(path, &cwd)
    };

    let path_in_fs = full_path.strip_prefix("/").unwrap_or(&full_path);

    let root_dir = ROOT_DIR.exclusive_access();

    let maybe_inode = if flags.contains(OpenFlags::CREATE) {
        root_dir
            .open_file(path_in_fs)
            .or_else(|_| root_dir.create_file(path_in_fs))
            .ok()
    } else {
        root_dir.open_file(path_in_fs).ok()
    };

    maybe_inode.map(|mut inode| {
        if flags.contains(OpenFlags::TRUNC) {
            inode.truncate().expect("Truncation failed");
        }
        Arc::new(OSInode::new(readable, writable, FatType::File(inode)))
    })
}

pub fn open_dir(path: &str) -> Result<(), ()> {
    let full_path = {
        let proc = current_process();
        let inner = proc.inner_exclusive_access();
        let cwd = &inner.cwd;
        resolve_path(path, &cwd)
    };
    
    let path_in_fs = full_path.strip_prefix("/").unwrap_or(&full_path);

    let root_dir = ROOT_DIR.exclusive_access();

    root_dir.open_dir(path_in_fs).map(|_| ()).map_err(|_| ())
}
