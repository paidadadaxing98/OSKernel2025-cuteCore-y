use crate::hal::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;
use core::ptr::addr_of_mut;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    println!("Heap allocation error:");
    println!("  Requested size: {} bytes", layout.size());
    println!("  Requested alignment: {} bytes", layout.align());
    println!("  Total heap size: {} bytes", KERNEL_HEAP_SIZE);
    println!(
        "  Requested size in MB: {:.2} MB",
        layout.size() as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Total heap size in MB: {:.2} MB",
        KERNEL_HEAP_SIZE as f64 / (1024.0 * 1024.0)
    );

    panic!("Heap allocation error, layout = {:?}", layout);
}

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, KERNEL_HEAP_SIZE);
    }
}
