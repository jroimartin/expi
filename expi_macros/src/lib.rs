//! Macros that generate low-level boilerplate code.
//!
//! expi does not aim to be used to build general purpose Operating Systems. We
//! typically run in EL2. Thus, the macros are limited to this use case.

use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Ident, ItemFn, Token};

/// Generates the boilerplate required to call the provided function on boot.
///
/// It also generates a panic handler and a global memory allocator. It tries
/// to initialize the global resources. If UART initialization fails, it enters
/// an infinite loop, otherwise it panics.
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

    let start_code = format!(
        r#"
                // Allocate an initial stack of approximately 0x80000 bytes.
                // This is a temporary stack used by `_globals_init`.
                ldr x5, =0x80000
                mov sp, x5

                // Save dtb_ptr32.
                str x0, [sp, #-8]!

                // Initialize globals and get stack top address.
                bl _expi_globals_init
                mov x5, x0

                // Restore dtb_ptr32.
                ldr x0, [sp], #8

                // Set stack pointer.
                mov sp, x5

                bl {fname_c}

            1:
                b 1b
        "#
    );

    let tokens = quote! {
        #[no_mangle]
        extern "C" fn _expi_globals_init(dtb_ptr32: u32) -> u64 {
            match expi::globals::init(dtb_ptr32) {
                Ok(_) => {}
                Err(expi::globals::Error::UartError(_)) => loop {},
                Err(err) => panic!("init error: {}", err),
            }

            let end = expi::globals::GLOBALS
                .free_memory()
                .lock()
                .as_mut()
                .expect("uninitialized allocator")
                .end()
                .expect("unknown stack top address");

            end + 1
        }

        #[link_section = ".entry"]
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn _start() -> ! {
            core::arch::asm!(#start_code, options(noreturn))
        }

        #[no_mangle]
        unsafe extern "C" fn #fname_c(dtb_ptr32: u32) {
            #fname_rust(dtb_ptr32)
        }

        #[global_allocator]
        static GLOBAL_ALLOCATOR: expi::mm::GlobalAllocator =
            expi::mm::GlobalAllocator;

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

/// Size of the stack allocated for each core.
const CORE_STACK_SIZE: usize = 32 * 1024 * 1024;

/// The multi-processing version of [`macro@entrypoint`].
///
/// It boots the four cores allocating a fixed size stack for each one.
#[proc_macro_attribute]
pub fn entrypoint_mp(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);

    let fname_rust = &item_fn.sig.ident;
    let fname_c = format_ident!("_expi_c_{}", fname_rust);

    let start_mp_code = format!(
        r#"
                // Load dtb_ptr32.
                ldr x5, =0x1000
                ldr x0, [x5]

                // Load stack top address.
                ldr x5, =0x1008
                ldr x1, [x5]

                // Load stack size.
                ldr x2, ={CORE_STACK_SIZE:#x}

                // Get core ID.
                mrs x5, mpidr_el1
                and x5, x5, #0xff

                // Set stack pointer.
                add x5, x5, #1
                mul x5, x5, x2
                sub x5, x1, x5
                mov sp, x5

                bl {fname_c}

            1:
                b 1b
        "#,
    );

    let tokens = quote! {
        #[no_mangle]
        extern "C" fn _expi_globals_init(dtb_ptr32: u32) -> u64 {
            match expi::globals::init(dtb_ptr32) {
                Ok(_) => {}
                Err(expi::globals::Error::UartError(_)) => loop {},
                Err(err) => panic!("init error: {}", err),
            }

            let end = expi::globals::GLOBALS
                .free_memory()
                .lock()
                .as_mut()
                .expect("uninitialized allocator")
                .end()
                .expect("unknown stack top address");

            end + 1
        }

        #[link_section = ".entry"]
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn _start() -> ! {
            core::arch::asm!(
                r#"
                    // Save dtb_ptr32 at 0x1000.
                    ldr x5, =0x1000
                    str x0, [x5]

                    // Allocate an initial stack of approximately 0x80000 bytes
                    // for core 0. This is a temporary stack used by
                    // `_globals_init`.
                    ldr x5, =0x80000
                    mov sp, x5

                    // Initialize globals and get stack top address.
                    bl _expi_globals_init

                    // Save stack top address at 0x1008.
                    ldr x5, =0x1008
                    str x0, [x5]

                    // All cores but core 0 are waiting for a wakeup event.
                    // Once the event is received, they jump to the address
                    // stored at 0xe0 (core 1), 0xe8 (core 2) and 0xf0 (core 3)
                    // if not zero. Implementation:
                    // https://github.com/raspberrypi/tools/blob/master/armstubs/armstub8.S
                    adr x5, _expi_start_mp
                    mov x6, #0xe0
                    str x5, [x6], #0x8
                    str x5, [x6], #0x8
                    str x5, [x6], #0x8

                    sev

                    b _expi_start_mp
                "#,
                options(noreturn))
        }

        #[no_mangle]
        #[naked]
        unsafe extern "C" fn _expi_start_mp() -> ! {
            core::arch::asm!(#start_mp_code, options(noreturn))
        }

        #[no_mangle]
        unsafe extern "C" fn #fname_c(dtb_ptr32: u32) {
            #fname_rust(dtb_ptr32)
        }

        #[global_allocator]
        static GLOBAL_ALLOCATOR: expi::mm::GlobalAllocator =
            expi::mm::GlobalAllocator;

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
            stp x0, x1, [sp, #-16]!
            stp x2, x3, [sp, #-16]!
            stp x4, x5, [sp, #-16]!
            stp x6, x7, [sp, #-16]!
            stp x8, x9, [sp, #-16]!
            stp x10, x11, [sp, #-16]!
            stp x12, x13, [sp, #-16]!
            stp x14, x15, [sp, #-16]!
            stp lr, xzr, [sp, #-16]!

            bl {fname_c}

            ldp lr, xzr, [sp], #16
            ldp x14, x15, [sp], #16
            ldp x12, x13, [sp], #16
            ldp x10, x11, [sp], #16
            ldp x8, x9, [sp], #16
            ldp x6, x7, [sp], #16
            ldp x4, x5, [sp], #16
            ldp x2, x3, [sp], #16
            ldp x0, x1, [sp], #16

            eret
        "#
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
/// - `curr_el_spx_sync`: The exception handler for a synchronous exception
///   from the current EL using the current SP.
/// - `curr_el_spx_irq`: The exception handler for an IRQ exception from the
///   current EL using the current SP.
/// - `curr_el_spx_fiq`: The exception handler for an FIQ from the current EL
///   using the current SP.
/// - `curr_el_spx_serror`: The exception handler for a System Error exception
///   from the current EL using the current SP.
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

    if fnames.len() != 4 {
        panic!(
            "the number of entries must be 4: {} entries provided",
            fnames.len()
        );
    }

    let fnames_asm = fnames
        .iter()
        .map(|p| format_ident!("_expi_asm_{}", p))
        .collect::<Vec<Ident>>();

    let vector_table_code = format!(
        r#"
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc

            .balign 0x80
            b {curr_el_spx_sync}
            .balign 0x80
            b {curr_el_spx_irq}
            .balign 0x80
            b {curr_el_spx_fiq}
            .balign 0x80
            b {curr_el_spx_serror}

            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc

            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc
            .balign 0x80
            b _expi_c_unimplemented_exc
        "#,
        curr_el_spx_sync = fnames_asm[0],
        curr_el_spx_irq = fnames_asm[1],
        curr_el_spx_fiq = fnames_asm[2],
        curr_el_spx_serror = fnames_asm[3],
    );

    let tokens = quote! {
        #[link_section = ".exception_vector_table"]
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn _exception_vector_table() -> ! {
            core::arch::asm!(#vector_table_code, options(noreturn))
        }

        #[no_mangle]
        unsafe extern "C" fn _expi_c_unimplemented_exc() {
            unimplemented!();
        }
    };

    tokens.into()
}
