pub mod config;
pub mod kernel_stack;
pub mod trap;
pub mod sbi;
pub mod sync;
pub mod timer;
mod boot;
mod laflex;
mod merrera;

mod tlb;

use crate::hal::platform::UART_BASE;
use config::{DIR_WIDTH, MMAP_BASE, PAGE_SIZE_BITS, PTE_WIDTH, PTE_WIDTH_BITS, SUC_DMW_VSEG};
use loongArch64::register::ecfg::LineBasedInterrupt;
use loongArch64::register::{cpuid, crmd, dmw0, dmw2, ecfg, euen, misc, prcfg1, pwch, pwcl, rvacfg, stlbps, tcfg, ticlr, tlbrehi, tlbrentry, MemoryAccessType};
use timer::get_timer_freq_first_time;
use trap::{set_kernel_trap_entry, set_machine_error_trap_entry};

extern "C" {
    pub fn srfill();
}

pub fn bootstrap_init() {
    // 如果不是0号核就死循环等待
    if cpuid::read().core_id() != 0 {
        loop {}
    };

    // ecfg：中断配置寄存器，开启定时器中断的局部使能
    ecfg::set_lie(LineBasedInterrupt::TIMER);

    // 开启浮点拓展
    euen::set_fpe(true);

    // 关闭定时器中断
    ticlr::clear_timer_interrupt();
    tcfg::set_en(false);

    crmd::set_we(false);    // 禁止 Wathpoint
    crmd::set_pg(true);     // 开启分页，进入48位虚拟地址模式
    crmd::set_ie(false);    // 禁用全局中断

    set_kernel_trap_entry();    // 设置内核态异常入口
    set_machine_error_trap_entry(); // 设置机器错误异常入口

    // tlbrentry 特指 TLB 重填异常 的入口地址（指向汇编编写的 srfill），当硬件查不到页表映射时，会跳转到这里进行快速重填。
    tlbrentry::set_tlbrentry(srfill as *const () as usize);

    // dmw2：数据存储器窗口2，映射设备地址空间
    dmw2::set_plv0(true);
    dmw2::set_plv1(false);
    dmw2::set_plv2(false);
    dmw2::set_plv3(false);
    dmw2::set_vseg(SUC_DMW_VSEG);   // 虚拟地址的高三位为010，映射到虚拟空间0x2000
    dmw2::set_mat(MemoryAccessType::StronglyOrderedUnCached);   // 窗口禁止缓存

    // INFO: dmw3 npucore中实现了，但是新版LoongArch64库接口缺失

    stlbps::set_ps(PTE_WIDTH_BITS); // 这是普通页大小的设置
    tlbrehi::set_ps(PTE_WIDTH_BITS);    // 这是大页大小的设置

    // 设置页表项格式
    pwcl::set_ptbase(PTE_WIDTH_BITS);   // 设置页表项基址偏移
    pwcl::set_ptwidth(DIR_WIDTH);   // 索引宽度
    pwcl::set_dir1_base(PAGE_SIZE_BITS + DIR_WIDTH);    // 目录1基址偏移
    pwcl::set_dir1_width(DIR_WIDTH);    // 目录1索引宽度
    pwcl::set_dir2_base(0); // 目录2基址偏移
    pwcl::set_dir2_width(0);    // 目录2索引宽度
    pwcl::set_ptwidth(PTE_WIDTH);  // 每级页表项大小8字节

    pwch::set_dir3_base(PAGE_SIZE_BITS + DIR_WIDTH * 2);    // 目录3基址偏移
    pwch::set_dir3_width(DIR_WIDTH);    // 目录3索引宽度
    pwch::set_dir4_base(0); // 目录4基址偏移
    pwch::set_dir4_width(0);    // 目录4索引宽度

    println!("[kernel] UART address: {:#x}", UART_BASE);
    println!("[bootstrap_init] {:?}", prcfg1::read());
}

pub fn machine_init() {
    trap::init();
    get_timer_freq_first_time();
    /* println!(
     *     "[machine_init] VALEN: {}, PALEN: {}",
     *     cfg0.get_valen(),
     *     cfg0.get_palen()
     * ); */
    for i in 0..=6 {
        let j: usize;
        unsafe { core::arch::asm!("cpucfg {0},{1}",out(reg) j,in(reg) i) };
        println!("[CPUCFG {:#x}] {}", i, j);
    }
    for i in 0x10..=0x14 {
        let j: usize;
        unsafe { core::arch::asm!("cpucfg {0},{1}",out(reg) j,in(reg) i) };
        println!("[CPUCFG {:#x}] {}", i, j);
    }
    println!("{:?}", misc::read());
    println!("{:?}", rvacfg::read());
    println!("[machine_init] MMAP_BASE: {:#x}", MMAP_BASE);

    trap::enable_timer_interrupt();
}

pub type PageTableEntryImpl = laflex::LAFlexPageTableEntry;
pub type PageTableImpl = laflex::LAFlexPageTable;