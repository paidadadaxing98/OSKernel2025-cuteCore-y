pub mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod pagetable;

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}

pub use crate::mm::memory_set::{kernel_token, MapPermission, MemorySet, KERNEL_SPACE};
pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_alloc_more, frame_dealloc, FrameTracker};
pub use pagetable::{PageTable, UserBuffer, translated_byte_buffer, translated_str, translated_refmut, translated_ref};
