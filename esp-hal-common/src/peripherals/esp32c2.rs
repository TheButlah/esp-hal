use esp32c2 as pac;
// We need to export this for users to use
pub use pac::Interrupt;

// We need to export this in the hal for the drivers to use
pub(crate) use self::peripherals::*;

crate::peripherals! {
    APB_CTRL,
    APB_SARADC,
    ASSIST_DEBUG,
    DMA,
    ECC,
    EFUSE,
    EXTMEM,
    GPIO,
    I2C0,
    INTERRUPT_CORE0,
    IO_MUX,
    LEDC,
    RNG,
    RTC_CNTL,
    SENSITIVE,
    SHA,
    SPI0,
    SPI1,
    SPI2,
    SYSTEM,
    SYSTIMER,
    TIMG0,
    UART0,
    UART1,
    XTS_AES,
}
