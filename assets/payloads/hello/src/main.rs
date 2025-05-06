#![no_std]
#![no_main]

use core::arch::asm;

#[cfg(target_arch = "x86_64")]
mod consts {
    pub const SYS_EXIT: usize = 60;
    pub const SYS_WRITE: usize = 1;
}
#[cfg(target_arch = "aarch64")]
mod consts {
    pub const SYS_EXIT: usize = 93;
    pub const SYS_WRITE: usize = 64;
}
use consts::*;
const HELLO: &[u8] = include_bytes!("hello.txt");

fn exit(ret: isize) -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!(
        "syscall",
        in("rax") SYS_EXIT,
        in("rdi") ret,
        options(noreturn),
        );
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
        "svc #0",
        in("x8") SYS_EXIT,
        in("x0") ret,
        options(noreturn),
        );
    }
}

fn write(fd: usize, msg: *const u8, len: usize) -> isize {
    let ret: isize;

    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_WRITE,
            in("rdi") fd,
            in("rsi") msg,
            in("rdx") len,
            lateout("rax") ret,
        );
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            "adr x1, {msg}", // must use PC-relative addressing! which is natural in x86, but tricky here
            "svc #0",
            in("x8") SYS_WRITE,
            in("x0") fd,
            in("x2") len,
            lateout("x0") ret,
            msg = sym MSG
        );
    }

    ret
}

#[cfg(target_arch = "aarch64")]
#[link_section = ".text.msg"]
// see linker.ld : we must control where this symbol is placed otherwise it wont be reachable when elf headers are lost (raw binary )
#[no_mangle]
static MSG: [u8; HELLO.len()] = *include_bytes!("hello.txt");

#[no_mangle]
fn _start() {
    let _ = write(1, HELLO.as_ptr(), HELLO.len());
    exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
