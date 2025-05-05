#![no_std]
#![no_main]

use core::arch::asm;
const HELLO: &[u8] = include_bytes!("hello.txt");


#[cfg(target_arch = "x86_64")]
pub unsafe fn exit(ret: isize) -> ! {
    let sys_nr: usize = 60;
    asm!(
    "syscall",
    in("rax") sys_nr,
    in("rdi") ret,
    options(noreturn),
    );
}
#[cfg(target_arch = "aarch64")]
pub unsafe fn exit(ret: isize) -> ! {
    let sys_nr: usize = 93;
    asm!(
    "svc #0",
    in("x8") sys_nr,
    in("x0") ret,
    options(noreturn),
    );
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn write(fd: usize, msg: *const u8, len: usize) -> isize {
    let sys_nr: usize = 1;
    let ret: isize;
    asm!(
    "syscall",
    in("rax") sys_nr,
    in("rdi") fd,
    in("rsi") msg,
    in("rdx") len,
    lateout("rax") ret,
    );
    ret
}
#[cfg(target_arch = "aarch64")]
pub unsafe fn write(fd: usize, msg: *const u8, len: usize) -> isize {
    let sys_nr: usize = 64;
    let ret: isize;

    asm!(
    "adr x1, {msg}", // must use PC-relative addressing! which is natural in x86, but tricky here
    "svc #0",
    in("x8") sys_nr,
    in("x0") fd,
    in("x2") len,
    lateout("x0") ret,
    msg = sym MSG)

    ret
}

#[cfg(target_arch = "aarch64")]
#[link_section = ".text.msg"]
// see linker.ld : we must control where this symbol is placed otherwise it wont be reachable when elf headers are lost (raw binary )
#[no_mangle]
pub static MSG: [u8; HELLO.len()] = *include_bytes!("hello.txt");


#[no_mangle]
pub extern "C" fn _start() {
    unsafe {
        let _ = write(1, HELLO.as_ptr(), HELLO.len());
        exit(0);
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
