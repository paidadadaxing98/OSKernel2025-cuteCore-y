use alloc::vec;
use alloc::vec::Vec;
use bitflags::{bitflags, Flags};
use loongArch64::register::{pgdh, pgdl};
use crate::hal::{PageTableEntryImpl, MEMORY_HIGH_BASE, MEMORY_HIGH_BASE_VPN, PAGE_SIZE_BITS, PALEN, VPN_SEG_MASK};
use crate::hal::arch::loongarch::tlb::tlb_global_invalidate;
use crate::mm::{frame_alloc, FrameTracker, MapPermission, PageTable, PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};

bitflags! {
    #[derive(Eq, PartialEq)]
    pub struct PTEFlags: usize {
        // ------ 强制要求位(硬件要求) ------
        const V = 1 << 0;   // 有效位，表示该页表项是否有效
        const D = 1 << 1;   // 脏位，表示页是否被修改过
        const PLV0 = 0 << 2;    // 特权级别0，最高特权级，内核
        const PLV3 = 3 << 2;    // 特权级别3，最低特权级，用户
        const MAT_CC = 1 << 4;  // 内存访问类型：一致性缓存（Coherent Cached）
        const MAT_SUC = 0 << 4; // 内存访问类型：强顺序非缓存（Strongly-ordered UnCached）
        const G = 1 << 6;   // 全局位

        // ------ 自定义位(软件) ------
        const P = 1 << 7;   // 物理位，表示物理页是否存在
        const W = 1 << 8;   // 可写位

        // ------ 高位安全属性(软件) ------
        const NR = 1 << (usize::BITS-3); // 不可读位
        const NX = 1 << (usize::BITS-2); // 不可执行位
        const RPLV = 1 << (usize::BITS-1); // 限制特权级别使能位
    }
}


#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

const PPN_MASK: usize = ((1usize << PALEN) - 1) & !((1usize << 12) - 1);

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: ((ppn.0 << 12) & PPN_MASK) | flags.bits(),
        }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        PhysPageNum((self.bits & PPN_MASK) >> 12)
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V)
    }

    pub fn readable(&self) -> bool {
        !self.flags().contains(PTEFlags::NR)
    }

    pub fn writable(&self) -> bool {
        self.flags().contains(PTEFlags::W)
    }

    pub fn executable(&self) -> bool {
        !self.flags().contains(PTEFlags::NX)
    }

    pub fn set_dirty(&mut self) {
        self.bits |= PTEFlags::D.bits();
    }

    pub fn is_dirty(&self) -> bool {
        self.flags().contains(PTEFlags::D)
    }

    pub fn clear_dirty(&mut self) {
        self.bits &= !PTEFlags::D.bits();
    }

    pub fn set_permission(&mut self, flags: MapPermission) {
        if flags.contains(MapPermission::R) {
            self.bits &= !PTEFlags::NR.bits();
        } else {
            self.bits |= PTEFlags::NR.bits();
        }

        if flags.contains(MapPermission::X) {
            self.bits &= !PTEFlags::NX.bits();
        } else {
            self.bits |= PTEFlags::NX.bits();
        }

        if flags.contains(MapPermission::W) {
            self.bits |= PTEFlags::W.bits();
        } else {
            self.bits &= !PTEFlags::W.bits();
        }
    }
}

pub struct LaflexPageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl LaflexPageTable {

    fn is_ident_map(&self, vpn: VirtPageNum) -> bool {
        self.is_kernel_pt() & (vpn.0 & VPN_SEG_MASK == MEMORY_HIGH_BASE_VPN)
    }

    fn is_kernel_pt(&self) -> bool {
        (self.token() as u32) == 0
    }

    fn get_root_ppn(&self) -> PhysPageNum {
        if self.is_kernel_pt() {
            PhysPageNum(self.root_ppn.0 >> 32)
        } else {
            self.root_ppn
        }
    }
}

impl PageTable for LaflexPageTable {
    /// 仅能用于创建用户页表
    fn new() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 仅能用于创建内核页表
    fn new_kernel() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_ppn: PhysPageNum(frame.ppn.0 << 32),
            frames: vec![frame],
        }
    }

    fn from_token(token: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(token),
            frames: Vec::new(),
        }
    }

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntryImpl> {
        let idex = vpn.indexes::<4>();
        let mut ppn = self.get_root_ppn();
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idex) in idex.iter().enumerate() {
            let pte = &mut ppn.get_pte_array::<PageTableEntry>()[*idex];
            if i == 3 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = PhysAddr::from((pte.ppn().0 << 12) | MEMORY_HIGH_BASE).floor();
        }
        result
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntryImpl> {
        let idxs = vpn.indexes::<4>();
        let mut ppn = self.get_root_ppn();
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte= &mut ppn.get_pte_array::<PageTableEntry>()[*idx];
            if i == 3 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = PhysAddr::from((pte.ppn().0 << 12) | MEMORY_HIGH_BASE).floor();
        }
    }

    fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: MapPermission) {
        let pte = self.find_pte_create(vpn).unwrap();
        let mut flag = PTEFlags::V | PTEFlags::MAT_CC;
        if !flags.contains(MapPermission::R) {
            flag |= PTEFlags::NR;
        }
        if !flags.contains(MapPermission::X) {
            flag |= PTEFlags::NX;
        }
        if flags.contains(MapPermission::W) {
            flag |= PTEFlags::W;
        }
        if flags.contains(MapPermission::U) {
            flag |= PTEFlags::PLV3;
        }
        let pte_new = PageTableEntry::new(ppn, flag);
        *pte = pte_new;
    }

    fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is unmapped before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntryImpl> {
        self.find_pte(vpn).map(|pte: PageTableEntry| *pte)
    }

    fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte: PageTableEntry| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

    fn activate(&self) {
        tlb_global_invalidate();
        if self.is_kernel_pt() {
            pgdh::set_base(self.get_root_ppn().0 << PAGE_SIZE_BITS);
        } else {
            pgdl::set_base(self.get_root_ppn().0 << PAGE_SIZE_BITS);
        }

    }

    fn token(&self) -> usize {
        self.root_ppn.0
    }
}