//! Local timer driver.

use crate::mmio;

/// Base address of the ARM-local peripherals.
///
/// [/arch/arm/boot/dts/bcm2837.dtsi] describes it:
///
/// ```text
/// local_intc: local_intc@40000000 {
///     compatible = "brcm,bcm2836-l1-intc";
///     reg = <0x40000000 0x100>;
///     ...
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm2837.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm2837.dtsi#L13-L19
const LOCAL_BASE: usize = 0x100_0000;

/// Local timer control and status.
const LOCAL_TIMER_CONTROL_STATUS: usize = LOCAL_BASE + 0x34;

/// Local timer IRQ clear and reload.
const LOCAL_TIMER_IRQ_CLEAR_RELOAD: usize = LOCAL_BASE + 0x38;

/// Enables the local timer.
pub fn enable() {
    let mut val = unsafe { mmio::read(LOCAL_TIMER_CONTROL_STATUS) };
    val |= 1 << 28;
    unsafe { mmio::write(LOCAL_TIMER_CONTROL_STATUS, val) };
}

/// Sets the reload value of the local timer. The value must be a 28-bit
/// unsigned integer. The 4 most significant bits are ignored. It acts as a
/// frequency divider.
pub fn set_reload_value(reload: u32) {
    let mut val = unsafe { mmio::read(LOCAL_TIMER_CONTROL_STATUS) };
    let mask = 0xfff_ffff;
    val = (val & !mask) | (reload & mask);
    unsafe { mmio::write(LOCAL_TIMER_CONTROL_STATUS, val) };
}

/// Clears interrupt.
pub fn clear() {
    unsafe { mmio::write(LOCAL_TIMER_IRQ_CLEAR_RELOAD, 1 << 31) };
}
