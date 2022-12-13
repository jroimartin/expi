//! Macros that generate low-level boilerplate code.

use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Ident, ItemFn, Token};

/// Generates the boilerplate required to call the provided function on boot.
///
/// It also generates a panic handler and tries to initialize the UART, so
/// panic messages can be printed. If UART initialization fails, it enters an
/// infinite loop.
///
/// Under the hood it specifies that the entrypoint must be placed into a
/// section called `.entry`.
///
/// The Raspberry Pi 3 Model B expects the entrypoint of the kernel to be at
/// 0x80000. Therefore, we need the linker to place the section `.entry` at
/// this address.
///
/// The following example shows how to do this using a Cargo configuration
/// file.
///
/// ```text
/// [target.aarch64-unknown-none]
/// rustflags = [
///     "-Clink-arg=--image-base=0x80000",
///     "-Clink-arg=--section-start=.entry=0x80000",
/// ]
/// ```
#[proc_macro_attribute]
pub fn entrypoint(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);

    let fname_rust = &item_fn.sig.ident;
    let fname_c = format_ident!("_expi_c_{}", fname_rust);

    let entry_code = format!(
        r#"
                ldr x5, =0x80000
                mov sp, x5
                bl {}
            1:
                b 1b
        "#,
        fname_c,
    );

    let tokens = quote! {
        #[link_section = ".entry"]
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn _start() -> ! {
            core::arch::asm!(#entry_code, options(noreturn))
        }

        #[no_mangle]
        unsafe extern "C" fn #fname_c() {
            if expi::uart::init().is_err() {
                loop{}
            }

            #fname_rust()
        }

        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            expi::print!("\n\n!!! PANIC !!!\n\n");

            if let Some(location) = info.location() {
                expi::print!("{}:{}", location.file(), location.line());
            }

            if let Some(message) = info.message() {
                expi::println!(": {}", message);
            } else {
                expi::println!();
            }

            loop {}
        }

        #item_fn
    };

    tokens.into()
}

/// Generates the boilerplate required to call the provided function as an
/// exception handler.
#[proc_macro_attribute]
pub fn exception_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);

    let fname_rust = &item_fn.sig.ident;
    let fname_asm = format_ident!("_expi_asm_{}", fname_rust);
    let fname_c = format_ident!("_expi_c_{}", fname_rust);

    let handler_code = format!(
        r#"
            STP X0, X1, [SP, #-16]!
            STP X2, X3, [SP, #-16]!
            STP X4, X5, [SP, #-16]!
            STP X6, X7, [SP, #-16]!
            STP X8, X9, [SP, #-16]!
            STP X10, X11, [SP, #-16]!
            STP X12, X13, [SP, #-16]!
            STP X14, X15, [SP, #-16]!

            bl {}

            LDP X14, X15, [SP], #16
            LDP X12, X13, [SP], #16
            LDP X10, X11, [SP], #16
            LDP X8, X9, [SP], #16
            LDP X6, X7, [SP], #16
            LDP X4, X5, [SP], #16
            LDP X2, X3, [SP], #16
            LDP X0, X1, [SP], #16

            eret
        "#,
        fname_c,
    );

    let tokens = quote! {
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn #fname_asm() -> ! {
            core::arch::asm!(#handler_code, options(noreturn))
        }

        #[no_mangle]
        unsafe extern "C" fn #fname_c() {
            #fname_rust()
        }

        #item_fn
    };

    tokens.into()
}

/// Represents the parameters of the [exception_vector_table] macro.
struct ExceptionVectorTableParams(Punctuated<Ident, Token![,]>);

impl Parse for ExceptionVectorTableParams {
    fn parse(input: ParseStream) -> syn::Result<ExceptionVectorTableParams> {
        let params = Punctuated::parse_terminated(input)?;
        Ok(ExceptionVectorTableParams(params))
    }
}

/// Generates an exception vector table.
///
/// It takes the following arguments:
///
/// - `curr_el_sp0_sync`: The exception handler for a synchronous exception
///   from the current EL using SP0.
/// - `curr_el_sp0_irq`: The exception handler for an IRQ exception from the
///   current EL using SP0.
/// - `curr_el_sp0_fiq`: The exception handler for an FIQ exception from the
///   current EL using SP0.
/// - `curr_el_sp0_serror`: The exception handler for a system error exception
///   from the current EL using SP0.
/// - `curr_el_spx_sync`: The exception handler for a synchronous exception
///   from the current EL using the current SP.
/// - `curr_el_spx_irq`: The exception handler for an IRQ exception from the
///   current EL using the current SP.
/// - `curr_el_spx_fiq`: The exception handler for an FIQ from the current EL
///   using the current SP.
/// - `curr_el_spx_serror`: The exception handler for a System Error exception
///   from the current EL using the current SP.
/// - `lower_el_aarch64_sync`: The exception handler for a synchronous
///   exception from a lower EL (AArch64).
/// - `lower_el_aarch64_irq`: The exception handler for an IRQ from a lower EL
///   (AArch64).
/// - `lower_el_aarch64_fiq`: The exception handler for an FIQ from a lower EL
///   (AArch64).
/// - `lower_el_aarch64_serror`: The exception handler for a System Error
///   exception from a lower EL (AArch64).
/// - `lower_el_aarch32_sync`: The exception handler for a synchronous
///   exception from a lower EL (AArch32).
/// - `lower_el_aarch32_irq`: The exception handler for an IRQ exception from a
///   lower EL (AArch32).
/// - `lower_el_aarch32_fiq`: The exception handler for an FIQ exception from a
///   lower EL (AArch32).
/// - `lower_el_aarch32_serror`: The exception handler for a System Error
///   exception from a lower EL (AArch32).
///
/// Under the hood it creates a symbol called `_exception_vector_table` and
/// specifies that it must be placed into a section called
/// `.exception_vector_table`.
///
/// Given that vector tables are usually not referenced by other code, we need
/// to ensure that the linker does not optimize them away. This can be done
/// with the linker flag `--undefined`, which forces the symbol to be entered
/// in the output file as an undefined symbol.
///
/// It is also necessary to set the location of the vector table in memory.
///
/// The following example shows how to do this using a Cargo configuration
/// file. It places the vector table at 0x90000.
///
/// ```text
/// [target.aarch64-unknown-none]
/// rustflags = [
///     "-Clink-arg=--undefined=_exception_vector_table",
///     "-Clink-arg=--section-start=.exception_vector_table=0x90000",
/// ]
/// ```
#[proc_macro]
pub fn exception_vector_table(item: TokenStream) -> TokenStream {
    let fnames = parse_macro_input!(item as ExceptionVectorTableParams);
    let fnames = fnames.0;

    if fnames.len() != 16 {
        panic!(
            "the number of entries must be 16 ({} provided)",
            fnames.len()
        );
    }

    let fnames_asm = fnames
        .iter()
        .map(|p| format_ident!("_expi_asm_{}", p))
        .collect::<Vec<Ident>>();

    let vector_table_code = format!(
        r#"
            b {curr_el_sp0_sync}
            .balign 0x80
            b {curr_el_sp0_irq}
            .balign 0x80
            b {curr_el_sp0_fiq}
            .balign 0x80
            b {curr_el_sp0_serror}

            .balign 0x80
            b {curr_el_spx_sync}
            .balign 0x80
            b {curr_el_spx_irq}
            .balign 0x80
            b {curr_el_spx_fiq}
            .balign 0x80
            b {curr_el_spx_serror}

            .balign 0x80
            b {lower_el_aarch64_sync}
            .balign 0x80
            b {lower_el_aarch64_irq}
            .balign 0x80
            b {lower_el_aarch64_fiq}
            .balign 0x80
            b {lower_el_aarch64_serror}

            .balign 0x80
            b {lower_el_aarch32_sync}
            .balign 0x80
            b {lower_el_aarch32_irq}
            .balign 0x80
            b {lower_el_aarch32_fiq}
            .balign 0x80
            b {lower_el_aarch32_serror}
        "#,
        curr_el_sp0_sync = fnames_asm[0],
        curr_el_sp0_irq = fnames_asm[1],
        curr_el_sp0_fiq = fnames_asm[2],
        curr_el_sp0_serror = fnames_asm[3],
        curr_el_spx_sync = fnames_asm[4],
        curr_el_spx_irq = fnames_asm[5],
        curr_el_spx_fiq = fnames_asm[6],
        curr_el_spx_serror = fnames_asm[7],
        lower_el_aarch64_sync = fnames_asm[8],
        lower_el_aarch64_irq = fnames_asm[9],
        lower_el_aarch64_fiq = fnames_asm[10],
        lower_el_aarch64_serror = fnames_asm[11],
        lower_el_aarch32_sync = fnames_asm[12],
        lower_el_aarch32_irq = fnames_asm[13],
        lower_el_aarch32_fiq = fnames_asm[14],
        lower_el_aarch32_serror = fnames_asm[15],
    );

    let tokens = quote! {
        #[link_section = ".exception_vector_table"]
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn _exception_vector_table() -> ! {
            core::arch::asm!(#vector_table_code, options(noreturn))
        }
    };

    tokens.into()
}
