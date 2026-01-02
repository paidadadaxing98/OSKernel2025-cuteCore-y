pub mod block_dev;
mod virtio_blk_mmio;

use alloc::sync::Arc;
use block_dev::BlockDevice;
use lazy_static::lazy_static;
use virtio_blk_mmio::VirtIOBlock;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(VirtIOBlock::new());
}
