//! # 块缓存模块（Block Cache Module）
//!
//! ## Overview
//! 本模块实现了一个基于内存的块缓存系统，用于缓存底层块设备（`BlockDevice`）中的固定大小数据块。
//! 其目标是：
//! - 减少对块设备的频繁 I/O 操作
//! - 提供对块内任意偏移位置的类型安全访问
//! - 在缓存被修改时延迟写回（write-back）
//!
//! 模块核心由以下几部分组成：
//! - `CacheData`：负责以 `BLOCK_SZ` 对齐方式管理原始块数据内存
//! - `BlockCache`：表示单个块的缓存实例
//! - `BlockCacheManager`：统一管理多个块缓存，提供简单的替换策略
//!
//! ## Assumptions
//! - 所有块大小均为常量 `BLOCK_SZ`
//! - 块设备的 `read_block` / `write_block` 能正确处理大小为 `BLOCK_SZ` 的缓冲区
//! - 上层调用者在使用 `get_ref` / `get_mut` 时，确保偏移与类型布局的正确性
//!
//! ## Safety
//! - 本模块内部大量使用 `unsafe`，主要集中在：
//!   - 手动内存分配与释放
//!   - 原始指针到引用的转换
//! - 所有 `unsafe` 均通过边界检查（offset + size <= BLOCK_SZ）
//!   与模块级不变量保证其安全性
//!
//! ## Invariants
//! - `CacheData` 持有的内存始终满足：
//!   - 大小为 `BLOCK_SZ`
//!   - 对齐方式为 `BLOCK_SZ`
//! - `BlockCache.modified == true` 表示缓存数据与磁盘不一致
//! - 被淘汰（drop）的 `BlockCache` 一定会在必要时写回磁盘
//!
//! ## Behavior
//! - 缓存采用固定容量（`BLOCK_CACHE_SIZE`）
//! - 当缓存满时，优先回收 `Arc` 强引用计数为 1 的缓存块
//! - 若无可回收缓存块，则直接 panic

use crate::drivers::BlockDevice;
use crate::hal::BLOCK_SZ;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::mem::ManuallyDrop;
use core::ptr::{addr_of, addr_of_mut};
use core::slice;
use lazy_static::*;
use spin::Mutex;

/// 使用 `ManuallyDrop` 确保数据以 `BLOCK_SZ` 对齐方式分配和释放
///
/// ## Overview
/// `CacheData` 封装了一个固定大小的块缓冲区，
/// 通过手动控制内存分配与释放来保证特殊的对齐要求。
///
/// ## Safety
/// - 内部通过 `alloc` / `dealloc` 手动管理内存
/// - 禁止默认的 `Box` drop 行为，避免使用错误的对齐方式释放
struct CacheData(ManuallyDrop<Box<[u8; BLOCK_SZ]>>);

impl CacheData {
    /// 创建新的缓存数据块
    ///
    /// ## Behavior
    /// - 使用自定义 `Layout` 分配内存
    /// - 保证大小和对齐方式均为 `BLOCK_SZ`
    pub fn new() -> Self {
        let data = unsafe {
            let raw = alloc::alloc::alloc(Self::layout());
            Box::from_raw(raw as *mut [u8; BLOCK_SZ])
        };
        Self(ManuallyDrop::new(data))
    }

    /// 返回缓存数据的内存布局描述
    ///
    /// ## Invariants
    /// - size == BLOCK_SZ
    /// - align == BLOCK_SZ
    fn layout() -> Layout {
        Layout::from_size_align(BLOCK_SZ, BLOCK_SZ).unwrap()
    }
}

impl Drop for CacheData {
    /// 手动释放缓存数据内存
    ///
    /// ## Safety
    /// - 必须与 `layout()` 使用完全一致的参数释放
    fn drop(&mut self) {
        let ptr = self.0.as_mut_ptr();
        unsafe { alloc::alloc::dealloc(ptr, Self::layout()) };
    }
}

impl AsRef<[u8]> for CacheData {
    /// 以不可变切片形式访问缓存数据
    fn as_ref(&self) -> &[u8] {
        let ptr = self.0.as_ptr() as *const u8;
        unsafe { slice::from_raw_parts(ptr, BLOCK_SZ) }
    }
}

impl AsMut<[u8]> for CacheData {
    /// 以可变切片形式访问缓存数据
    fn as_mut(&mut self) -> &mut [u8] {
        let ptr = self.0.as_mut_ptr() as *mut u8;
        unsafe { slice::from_raw_parts_mut(ptr, BLOCK_SZ) }
    }
}

/// 内存中的单个块缓存
///
/// ## Fields
/// - `cache`：实际的块数据
/// - `block_id`：对应的磁盘块号
/// - `block_device`：底层块设备
/// - `modified`：是否被修改过
///
/// ## Invariants
/// - 若 `modified == true`，则缓存数据尚未写回磁盘
pub struct BlockCache {
    /// 缓存的块数据
    cache: CacheData,
    /// 对应的磁盘块编号
    block_id: usize,
    /// 关联的块设备
    block_device: Arc<dyn BlockDevice>,
    /// 是否被修改
    modified: bool,
}

impl BlockCache {
    /// 从磁盘加载一个新的块缓存
    ///
    /// ## Behavior
    /// - 分配新的缓存内存
    /// - 从块设备中读取指定块
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        // for alignment and move effciency
        let mut cache = CacheData::new();
        block_device.read_block(block_id, cache.as_mut());
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    /// 获取块内指定偏移处的原始地址（只读）
    fn addr_of_offset(&self, offset: usize) -> *const u8 {
        addr_of!(self.cache.as_ref()[offset])
    }

    /// 获取块内指定偏移处的原始地址（可写）
    fn addr_of_offset_mut(&mut self, offset: usize) -> *mut u8 {
        addr_of_mut!(self.cache.as_mut()[offset])
    }

    /// 获取指定偏移处的类型引用
    ///
    /// ## Safety
    /// - 调用者必须保证 `T` 在该偏移处布局正确
    /// - 本函数仅检查越界，不检查对齐与语义合法性
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset) as *const T;
        unsafe { &*addr }
    }

    /// 获取指定偏移处的可变类型引用
    ///
    /// ## Behavior
    /// - 自动将 `modified` 标记为 true
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset_mut(offset) as *mut T;
        unsafe { &mut *addr }
    }

    /// 只读访问接口，使用闭包封装
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    /// 可写访问接口，使用闭包封装
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    /// 将缓存数据同步写回磁盘
    ///
    /// ## Behavior
    /// - 仅当数据被修改过才会写回
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device
                .write_block(self.block_id, self.cache.as_ref());
        }
    }
}

impl Drop for BlockCache {
    /// 在缓存被丢弃时自动同步数据
    fn drop(&mut self) {
        self.sync()
    }
}

const BLOCK_CACHE_SIZE: usize = 16;

/// 块缓存管理器
///
/// ## Overview
/// 负责统一管理所有块缓存，并实现简单的缓存替换策略。
///
/// ## Fields
/// - `queue`：FIFO 队列，保存 `(block_id, BlockCache)`
///
/// ## Behavior
/// - 查找命中直接返回
/// - 未命中则可能触发缓存替换
pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    /// 创建新的缓存管理器
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// 获取指定块的缓存
    ///
    /// ## Behavior
    /// - 若缓存存在则直接返回
    /// - 若缓存已满，则回收引用计数为 1 的缓存块
    /// - 若无可回收缓存块，则 panic
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            Arc::clone(&pair.1)
        } else {
            // substitute
            if self.queue.len() == BLOCK_CACHE_SIZE {
                // from front to tail
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            // load block into mem and push back
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    /// 全局块缓存管理器实例
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

/// 获取指定块的缓存（全局接口）
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}

/// 同步所有缓存块到磁盘
///
/// ## Behavior
/// - 遍历当前缓存队列
/// - 对每个缓存执行 `sync`
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
