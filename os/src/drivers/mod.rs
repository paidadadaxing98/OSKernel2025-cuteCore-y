mod block;
pub mod serial;

pub use block::block_dev::BlockDevice;
pub use block::BLOCK_DEVICE;
pub use serial::ns16550a::Ns16550a;
