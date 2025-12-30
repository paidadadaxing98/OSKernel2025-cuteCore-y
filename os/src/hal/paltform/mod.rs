
#[cfg(feature = "board_rvqemu")]
pub mod riscv;

#[cfg(feature = "board_rvqemu")]
pub use riscv::qemu::*;