//! SV39 页表管理模块
//!
//! # Overview
//! 本模块实现 RISC-V SV39 虚拟内存页表管理，包括页表条目、页表创建、虚拟页映射、解除映射、查找以及页表激活功能。
//! 模块主要服务于操作系统内核虚拟内存管理，支持内核和用户空间页映射、权限管理以及地址转换。
//!
//! # Design
//! - 页表采用 SV39 三级页表结构，每级 9 位索引。
//! - 每次访问或映射虚拟页时，会自动创建缺失的页表页（Frame）。
//! - 每个页表条目（PTE）包含物理页号（PPN）和标记位（PTEFlags）。
//! - 页表使用 `frames` 记录当前分配的物理页，用于生命周期管理。
//! - 映射操作保证不会覆盖已存在的有效映射。
//!
//! # Assumptions
//! - 物理页分配（frame_alloc）不会失败，不考虑 OOM。
//! - 虚拟地址、物理地址及权限标记均合法。
//! - 系统遵循 RISC-V SV39 页表规范。
//!
//! # Safety
//! - 修改页表条目涉及物理内存访问，必须确保 PPn 和标记位合法。
//! - 页表激活时修改 SATP 寄存器，必须保证在允许上下文执行。
//! - 查找、映射、解除映射操作必须保证不会破坏已分配页表结构。
//!
//! # Invariants
//! - 每个虚拟页最多映射到一个物理页。
//! - 已分配的 Frame 在生命周期内不会重复释放。
//! - 页表条目有效性（V 位）与权限位保持一致。
//! - 激活页表后，SATP 寄存器反映根页表地址，并完成 TLB 同步。

use crate::mm::{
    frame_alloc, FrameTracker, MapPermission, PageTable, PhysAddr, PhysPageNum, VirtAddr,
    VirtPageNum,
};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;
use core::arch::asm;
use riscv::register::satp;

bitflags! {

    /// 页表条目标记
    #[derive(Eq, PartialEq)]
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

/// 单个页表条目
///
/// # Overview
/// 封装 SV39 页表条目（PTE），包含物理页号和标记位。
///
/// # Design
/// - bits 高 54 位存储物理页号 (PPN)
/// - bits 低 10 位存储标记位 (PTEFlags)
///
/// # Fields
/// - `bits`：页表条目的原始数据
///
/// # Assumptions
/// - 页表条目符合 SV39 页表规范
///
/// # Safety
/// - 修改 bits 必须确保 PPn 与标记位合法
///
/// # Invariants
/// - 有效的 PTE 必须 V 标志位为 1
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    /// 创建一个新的 PTE
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits() as usize,
        }
    }

    /// 创建空的 PTE
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }

    /// 获取物理页号
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    /// 获取 PTE 标记位
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// 判断 PTE 是否有效
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    /// 判断页是否可读
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    /// 判断页是否可写
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    /// 判断页是否可执行
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// SV39 页表实现
///
/// # Overview
/// 实现 RISC-V SV39 虚拟内存页表，提供页映射、查找、创建、激活等功能。
///
/// # Design
/// - 页表使用 3 级索引（SV39: 9+9+9）
/// - 每次访问或映射页时，会创建新的页表页（Frame）
/// - 使用 `frames` 保存分配的物理页，以便生命周期管理
///
/// # Fields
/// - `root_ppn`：根页表物理页号
/// - `frames`：当前页表使用的物理页集合
///
/// # Assumptions
/// - frame_alloc() 分配成功，不考虑 OOM
///
/// # Safety
/// - 访问物理页和修改 PTE 时需要保证合法性
///
/// # Invariants
/// - 每个虚拟页有唯一映射
/// - 分配的 Frame 在生命周期内不会被重复释放
pub struct SV39PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl PageTable for SV39PageTable {
    /// 创建新的空页表
    fn new() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 创建内核页表
    fn new_kernel() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 从 SATP 获取页表对象（临时用于用户态参数）
    fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// 查找或创建页表条目
    ///
    /// # Design
    /// 尝试查找 vpn 对应的物理 pte，若 pte 还没有创建就先创建
    ///
    /// # Reture
    /// vpn 对应的 pte
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes::<3>();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array::<PageTableEntry>()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }

    /// 查找页表条目
    ///
    /// # Design
    /// 尝试查找 vpn 对应的 pte， 若 pte 不存在则返回 None
    ///
    /// # Return
    /// vpn 对应的 pte 或 None
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes::<3>();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array::<PageTableEntry>()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    /// 映射虚拟页到物理页
    fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: MapPermission) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(
            ppn,
            PTEFlags::from_bits(flags.bits()).unwrap() | PTEFlags::V,
        );
    }

    /// 解除虚拟页映射
    fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    /// 虚拟页号到页表条目转换
    fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    /// 虚拟地址到物理地址转换
    fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

    /// 激活当前页表
    fn activate(&self) {
        let satp = self.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }

    /// 获取页表 token
    fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}
