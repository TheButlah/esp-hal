#![no_std]

#[cfg(feature = "mcu-boot")]
use core::mem::size_of;

pub use embedded_hal as ehal;
#[cfg(feature = "embassy")]
pub use esp_hal_common::embassy;
#[doc(inline)]
pub use esp_hal_common::{
    aes,
    analog::adc::implementation as adc,
    clock,
    dma,
    dma::gdma,
    efuse,
    gpio,
    i2c,
    i2s,
    interrupt,
    ledc,
    macros,
    peripherals,
    prelude,
    pulse_control,
    sha,
    spi,
    system,
    systimer,
    timer,
    twai,
    uart,
    utils,
    Cpu,
    Delay,
    PulseControl,
    Rng,
    Rtc,
    Rwdt,
    Uart,
    UsbSerialJtag,
};

pub use self::gpio::IO;

/// Common module for analog functions
pub mod analog {
    pub use esp_hal_common::analog::{AvailableAnalog, SarAdcExt};
}

extern "C" {
    cfg_if::cfg_if! {
        if #[cfg(feature = "mcu-boot")] {
            // Functions from internal ROM
            fn cache_suspend_icache() -> u32;
            fn cache_resume_icache(val: u32);
            fn cache_invalidate_icache_all();
            fn cache_dbus_mmu_set(
                ext_ram: u32,
                vaddr: u32,
                paddr: u32,
                psize: u32,
                num: u32,
                fixed: u32,
            ) -> i32;
            fn cache_ibus_mmu_set(
                ext_ram: u32,
                vaddr: u32,
                paddr: u32,
                psize: u32,
                num: u32,
                fixed: u32,
            ) -> i32;

            /* IROM metadata:
             * - Destination address (VMA) for IROM region
             * - Flash offset (LMA) for start of IROM region
             * - Size of IROM region
             */
            static mut _image_irom_vma: u32;
            static mut _image_irom_lma: u32;
            static mut _image_irom_size: u32;

            /* DROM metadata:
             * - Destination address (VMA) for DROM region
             * - Flash offset (LMA) for start of DROM region
             * - Size of DROM region
             */
            static mut _image_drom_vma: u32;
            static mut _image_drom_lma: u32;
            static mut _image_drom_size: u32;
        }
    }

    // Boundaries of the .iram section
    static mut _srwtext: u32;
    static mut _erwtext: u32;
    static mut _irwtext: u32;

    // Boundaries of the .bss section
    static mut _ebss: u32;
    static mut _sbss: u32;

    // Boundaries of the rtc .bss section
    static mut _rtc_fast_bss_start: u32;
    static mut _rtc_fast_bss_end: u32;

    // Boundaries of the .rtc_fast.text section
    static mut _srtc_fast_text: u32;
    static mut _ertc_fast_text: u32;
    static mut _irtc_fast_text: u32;

    // Boundaries of the .rtc_fast.data section
    static mut _rtc_fast_data_start: u32;
    static mut _rtc_fast_data_end: u32;
    static mut _irtc_fast_data: u32;
}

#[cfg(feature = "mcu-boot")]
#[link_section = ".entry_addr"]
#[no_mangle]
#[used]
// Entry point address for the MCUboot image header
static ENTRY_POINT: unsafe fn() -> ! = start_hal;

#[cfg(feature = "direct-boot")]
#[doc(hidden)]
#[esp_riscv_rt::pre_init]
unsafe fn init() {
    r0::init_data(&mut _srwtext, &mut _erwtext, &_irwtext);

    r0::init_data(
        &mut _rtc_fast_data_start,
        &mut _rtc_fast_data_end,
        &_irtc_fast_data,
    );

    r0::init_data(&mut _srtc_fast_text, &mut _ertc_fast_text, &_irtc_fast_text);
}

#[cfg(feature = "mcu-boot")]
#[link_section = ".rwtext"]
unsafe fn configure_mmu() {
    const PARTITION_OFFSET: u32 = 0x10000;
    let app_irom_lma = PARTITION_OFFSET + ((&_image_irom_lma as *const u32) as u32);
    let app_irom_size = (&_image_irom_size as *const u32) as u32;
    let app_irom_vma = (&_image_irom_vma as *const u32) as u32;
    let app_drom_lma = PARTITION_OFFSET + ((&_image_drom_lma as *const u32) as u32);
    let app_drom_size = (&_image_drom_size as *const u32) as u32;
    let app_drom_vma = (&_image_drom_vma as *const u32) as u32;

    let autoload = cache_suspend_icache();
    cache_invalidate_icache_all();

    // Clear the MMU entries that are already set up, so the new app only has
    // the mappings it creates.

    const FLASH_MMU_TABLE: *mut u32 = 0x600c_5000 as *mut u32;
    const ICACHE_MMU_SIZE: usize = 0x200;
    const FLASH_MMU_TABLE_SIZE: usize = ICACHE_MMU_SIZE / size_of::<u32>();
    const MMU_TABLE_INVALID_VAL: u32 = 0x100;

    for i in 0..FLASH_MMU_TABLE_SIZE {
        FLASH_MMU_TABLE.add(i).write_volatile(MMU_TABLE_INVALID_VAL);
    }

    const MMU_BLOCK_SIZE: u32 = 0x0001_0000;
    const MMU_FLASH_MASK: u32 = !(MMU_BLOCK_SIZE - 1);

    let calc_mmu_pages = |size, vaddr| {
        (size + (vaddr - (vaddr & MMU_FLASH_MASK)) + MMU_BLOCK_SIZE - 1) / MMU_BLOCK_SIZE
    };

    let drom_lma_aligned = app_drom_lma & MMU_FLASH_MASK;
    let drom_vma_aligned = app_drom_vma & MMU_FLASH_MASK;
    let drom_page_count = calc_mmu_pages(app_drom_size, app_drom_vma);
    cache_dbus_mmu_set(
        0,
        drom_vma_aligned,
        drom_lma_aligned,
        64,
        drom_page_count,
        0,
    );

    let irom_lma_aligned = app_irom_lma & MMU_FLASH_MASK;
    let irom_vma_aligned = app_irom_vma & MMU_FLASH_MASK;
    let irom_page_count = calc_mmu_pages(app_irom_size, app_irom_vma);
    cache_ibus_mmu_set(
        0,
        irom_vma_aligned,
        irom_lma_aligned,
        64,
        irom_page_count,
        0,
    );

    let peripherals = peripherals::Peripherals::steal();
    peripherals.EXTMEM.icache_ctrl1.modify(|_, w| {
        w.icache_shut_ibus()
            .clear_bit()
            .icache_shut_dbus()
            .clear_bit()
    });

    cache_resume_icache(autoload);
}

#[allow(unreachable_code)]
#[export_name = "_mp_hook"]
#[doc(hidden)]
#[cfg_attr(feature = "mcu-boot", link_section = ".rwtext")]
pub fn mp_hook() -> bool {
    #[cfg(feature = "mcu-boot")]
    unsafe {
        configure_mmu();
    }

    unsafe {
        r0::zero_bss(&mut _rtc_fast_bss_start, &mut _rtc_fast_bss_end);
    }

    #[cfg(feature = "direct-boot")]
    return true;

    // no init data when using normal boot - but we need to zero out BSS
    unsafe {
        r0::zero_bss(&mut _sbss, &mut _ebss);
    }

    false
}

#[no_mangle]
extern "C" fn EspDefaultHandler(_interrupt: peripherals::Interrupt) {}
