use alloc::vec::Vec;
use lazy_static::lazy_static;
use virtio_drivers::VirtIOBlk;
use crate::drivers::block::block_dev::BlockDevice;
use crate::hal::PageTableImpl;
use crate::mm;
use crate::mm::{frame_alloc_more, frame_dealloc, kernel_token, FrameTracker, PageTable, StepByOne};
use crate::sync::UPIntrFreeCell;

const VIRTIO0: usize = 0x10001000;


pub struct VirtIOBlock(UPIntrFreeCell<VirtIOBlk<'static, VirtIOHal>>);

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0.exclusive_access().read_block(block_id, buf).expect("Error when reading VirtIOBlk");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0.exclusive_access().write_block(block_id, buf).expect("Error when writing VirtIOBlk");
    }
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            Self(UPIntrFreeCell::new(
                VirtIOBlk::<VirtIOHal>::new(&mut *(VIRTIO0 as *mut virtio_drivers::VirtIOHeader)).unwrap()
            ))
        }
    }
}

lazy_static! {
    static ref QUEUE_FRAMES: UPIntrFreeCell<Vec<FrameTracker>> =
        unsafe { UPIntrFreeCell::new(Vec::new()) };
}

pub struct VirtIOHal;

impl virtio_drivers::Hal for VirtIOHal {
    fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
        let trakcers = frame_alloc_more(pages);
        let ppn_base = trakcers.as_ref().unwrap().last().unwrap().ppn;
        QUEUE_FRAMES
            .exclusive_access()
            .append(&mut trakcers.unwrap());
        let pa: mm::PhysAddr = ppn_base.into();
        pa.0
    }

    fn dma_dealloc(paddr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        let mut ppn_base: mm::PhysPageNum = paddr.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    fn phys_to_virt(paddr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        paddr.into()
    }

    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        PageTableImpl::from_token(kernel_token()).translate_va(mm::VirtAddr::from(vaddr)).unwrap().into()
    }
}