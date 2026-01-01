pub mod serial;
mod block;

pub use serial::ns16550a::Ns16550a;
pub use block::block_dev::BlockDevice;
pub use block::BLOCK_DEVICE;
