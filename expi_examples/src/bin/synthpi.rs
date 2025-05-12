//! Simple Synth.

#![no_std]
#![no_main]

use expi::cpu;
use expi::cpu::exceptions::{self, Exception, Interrupt};
use expi::cpu::mp;
use expi::gpio::{self, Event, Function, Pin, PullState};
use expi::intc::{self, IrqSource};
use expi::local_intc::{self, IntSource, IntType};
use expi::local_timer;
use expi::println;
use expi_macros::{entrypoint_mp, exception_handler, exception_vector_table};
use mutex::TicketMutex;

/// The output pin is GPIO26.
const GPIO_OUT: usize = 26;

/// The frequency selection button is connected to GPIO16.
const GPIO_FREQ_BUTTON: usize = 16;

/// Stores if the output pin is set.
static OUT_SET: TicketMutex<Option<bool>> = TicketMutex::new(None);

/// Local timer reload value for 20Khz.
const RELOAD_20KHZ: u32 = (19.2e6 / 20e3) as u32;

/// Local timer reload value for 20Hz.
const RELOAD_20HZ: u32 = (19.2e6 / 20f32) as u32;

/// Kernel main function.
#[entrypoint_mp]
fn kernel_main() {
    println!("expi");

    match mp::core_id() {
        0 => configure_global(),
        1 => configure_timer(),
        n => println!("halting core {n}"),
    }
}

/// Configure global resources.
fn configure_global() {
    // Configure exceptions.
    configure_exceptions();

    // Configure GPIO output.
    let out = Pin::try_from(GPIO_OUT).unwrap();
    out.set_function(Function::Output);

    {
        let mut out_set = OUT_SET.lock();
        *out_set = Some(false);
    }

    // Configure the GPIO pin of the frequency selecction button.
    IrqSource::GPIO.enable();
    let freq_button = Pin::try_from(GPIO_FREQ_BUTTON).unwrap();
    freq_button.set_pull_state(PullState::Up);
    freq_button.set_function(Function::Input);
    freq_button.enable_event(Event::FallingEdge);

    loop {
        cpu::wfi();
    }
}

/// Configure local timer.
fn configure_timer() {
    // Configure exceptions.
    configure_exceptions();

    // Configure local timer.
    IntSource::LocalTimer
        .route(mp::core(), IntType::Irq)
        .unwrap();
    IntSource::LocalTimer.enable();
    local_timer::set_reload_value(RELOAD_20KHZ);
    local_timer::enable();

    loop {
        cpu::wfi();
    }
}

/// Configure exceptions.
fn configure_exceptions() {
    // Mask all interrupts.
    Interrupt::SError.mask();
    Interrupt::Irq.mask();
    Interrupt::Fiq.mask();
    Exception::Debug.mask();

    // Enable pysical IRQ and FIQ routing.
    Interrupt::Irq.route();
    Interrupt::Fiq.route();

    // Set vector table address.
    exceptions::set_vector_table(0x81000);

    // Unmask IRQs and FIQs.
    Interrupt::Irq.unmask();
    Interrupt::Fiq.unmask();
}

/// IRQ handler.
#[exception_handler]
fn irq_handler() {
    match mp::core_id() {
        0 => irq_handler_core0(),
        1 => irq_handler_core1(),
        _ => {}
    }
}

/// Core 0's IRQ handler.
fn irq_handler_core0() {
    let basic_status = intc::basic_status();
    if basic_status.pending_reg_2() {
        let gpu_status = intc::gpu_status();
        if gpu_status.pending(IrqSource::GPIO).unwrap() {
            gpio_handler();
        }
    }
}

/// Core 1's IRQ handler.
fn irq_handler_core1() {
    let status = local_intc::irq_status(mp::core());
    if status.pending_local_timer() {
        local_timer_handler();
        local_timer::clear();
    }
}

/// Local Timer IRQ handler.
fn local_timer_handler() {
    let out = Pin::try_from(GPIO_OUT).unwrap();
    let mut out_set = OUT_SET.lock();

    match *out_set {
        None => {}
        Some(set) => {
            if set {
                out.clear();
            } else {
                out.set();
            }
            *out_set = Some(!set);
        }
    }
}

/// GPIO IRQ handler.
fn gpio_handler() {
    let freq_button = Pin::try_from(GPIO_FREQ_BUTTON).unwrap();
    let events = gpio::events();
    if events.detected(freq_button) {
        next_freq();
        freq_button.clear_event();
    }
}

/// Select next frequency value.
fn next_freq() {
    let reload = local_timer::reload_value();
    let val = if reload < RELOAD_20HZ {
        ((reload as f32) * 1.25) as u32
    } else {
        RELOAD_20KHZ
    };
    local_timer::set_reload_value(val);
}

/// Unimplemented exception handler.
#[exception_handler]
fn unimplemented_handler() {
    unimplemented!();
}

exception_vector_table! {
    // Synchronous.
    unimplemented_handler,
    // IRQ.
    irq_handler,
    // FIQ.
    unimplemented_handler,
    // SError
    unimplemented_handler,
}
