use riscv::register::sstatus::{read, Sstatus, SPP};

// INFO: 因为实现浮点寄存器需要修改整个汇编代码，所以暂时注释掉

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct GeneralRegs {
    pub x: [usize; 32],
}
//
// #[repr(C)]
// #[derive(Debug, Default, Clone, Copy)]
// pub struct FloatRegs {
//     pub f: [usize; 32],
//     pub fcsr: usize,
// }

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapContext {
    pub general_regs: GeneralRegs,
    // pub float_regs: FloatRegs,
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_satp: usize,
    pub kernel_sp: usize,
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.general_regs.x[2] = sp;
    }

    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            general_regs: GeneralRegs::default(),
            // float_regs: FloatRegs::default(),
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}