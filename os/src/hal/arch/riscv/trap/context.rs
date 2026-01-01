use riscv::register::sstatus::{read, Sstatus, SPP};

// INFO: 因为实现浮点寄存器需要修改整个汇编代码，所以暂时注释掉

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct GeneralRegs {
    pub pc: usize,  // 0
    pub ra: usize,  // 1
    pub sp: usize,  // 2
    pub gp: usize,  // 3
    pub tp: usize,  // 4
    pub t0: usize,  // 5
    pub t1: usize,  // 6
    pub t2: usize,  // 7
    pub s0: usize,  // 8
    pub s1: usize,  // 9
    pub a0: usize,  // 10
    pub a1: usize,  // 11
    pub a2: usize,  // 12
    pub a3: usize,  // 13
    pub a4: usize,  // 14
    pub a5: usize,  // 15
    pub a6: usize,  // 16
    pub a7: usize,  // 17
    pub s2: usize,  // 18
    pub s3: usize,  // 19
    pub s4: usize,  // 20
    pub s5: usize,  // 21
    pub s6: usize,  // 22
    pub s7: usize,  // 23
    pub s8: usize,  // 24
    pub s9: usize,  // 25
    pub s10: usize, // 26
    pub s11: usize, // 27
    pub t3: usize,  // 28
    pub t4: usize,  // 29
    pub t5: usize,  // 30
    pub t6: usize,  // 31
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
        self.general_regs.sp = sp;
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
