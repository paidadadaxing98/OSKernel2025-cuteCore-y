pub mod block_dev;
mod virtio_blk_mmio;

use alloc::sync::Arc;
use lazy_static::lazy_static;
use virtio_blk_mmio::VirtIOBlock;
use block_dev::BlockDevice;


lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(VirtIOBlock::new());
}
