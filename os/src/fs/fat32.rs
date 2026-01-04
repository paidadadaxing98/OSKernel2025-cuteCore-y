use crate::drivers::{BlockDevice, BLOCK_DEVICE};
use crate::fs::{block_cache_sync_all, get_block_cache};
use crate::hal::BLOCK_SZ;
use alloc::boxed::Box;
use alloc::sync::Arc;
use fatfs::{IoBase, IoError, Read, Seek, SeekFrom, Write};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref FAT_FS: Mutex<fatfs::FileSystem<FatFsBlockDevice>> = Mutex::new({
        let fat_device = FatFsBlockDevice::new(BLOCK_DEVICE.clone());
        let fs = fatfs::FileSystem::new(fat_device, fatfs::FsOptions::new())
            .expect("Failed to mount FAT filesystem");
        fs
    });
}

pub struct FatFsBlockDevice {
    block_device: Arc<dyn BlockDevice>,
    offset: usize,
}

impl FatFsBlockDevice {
    pub fn new(block_device: Arc<dyn BlockDevice>) -> Self {
        Self {
            block_device,
            offset: 0,
        }
    }
}

#[derive(Debug)]
pub enum FatFsError {
    IoError,
    InvalidOffset,
}

impl IoError for FatFsError {
    fn is_interrupted(&self) -> bool {
        false
    }

    fn new_unexpected_eof_error() -> Self {
        FatFsError::IoError
    }

    fn new_write_zero_error() -> Self {
        FatFsError::IoError
    }
}

impl IoBase for FatFsBlockDevice {
    type Error = FatFsError;
}

impl Read for FatFsBlockDevice {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut read_size = 0;
        let mut current_offset = self.offset;

        while read_size < buf.len() {
            let block_id = current_offset / BLOCK_SZ;
            let offset_in_block = current_offset % BLOCK_SZ;
            let size_to_read = (buf.len() - read_size).min(BLOCK_SZ - offset_in_block);

            get_block_cache(block_id, self.block_device.clone())
                .lock()
                .read(0, |block_data: &[u8; BLOCK_SZ]| {
                    buf[read_size..read_size + size_to_read].copy_from_slice(
                        &block_data.as_ref()[offset_in_block..offset_in_block + size_to_read],
                    );
                });
            read_size += size_to_read;
            current_offset += size_to_read;
        }
        self.offset += read_size;
        Ok(read_size)
    }
}

impl Write for FatFsBlockDevice {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut write_size = 0;
        let mut current_offset = self.offset;

        while write_size < buf.len() {
            let block_id = current_offset / BLOCK_SZ;
            let offset_in_block = current_offset % BLOCK_SZ;
            let size_to_write = (buf.len() - write_size).min(BLOCK_SZ - offset_in_block);

            get_block_cache(block_id, self.block_device.clone())
                .lock()
                .modify(0, |block_data: &mut [u8; BLOCK_SZ]| {
                    block_data.as_mut()[offset_in_block..offset_in_block + size_to_write]
                        .copy_from_slice(&buf[write_size..write_size + size_to_write]);
                });
            write_size += size_to_write;
            current_offset += size_to_write;
        }
        self.offset += write_size;
        Ok(write_size)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        block_cache_sync_all();
        Ok(())
    }
}

impl Seek for FatFsBlockDevice {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let new_offset = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => self.offset as i64 + offset,
            SeekFrom::End(offset) => {
                // TODO: 暂时未能实现此功能
                return Err(FatFsError::InvalidOffset);
            }
        };

        if new_offset < 0 {
            return Err(FatFsError::InvalidOffset);
        }

        self.offset = new_offset as usize;
        Ok(new_offset as u64)
    }
}
