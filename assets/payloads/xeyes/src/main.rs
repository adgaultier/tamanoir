#![no_std]
#![no_main]

use core::arch::asm;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(target_arch = "x86_64")]
mod consts {
    pub const SYS_FORK: usize = 57;
    pub const SYS_EXECVE: usize = 59;
    pub const SYS_EXIT: usize = 60;
}
#[cfg(target_arch = "aarch64")]
mod consts {
    pub const SYS_CLONE: usize = 220;
    pub const CLONE_FLAGS: usize = 0;
    pub const SYS_EXECVE: usize = 221;
    pub const SYS_EXIT: usize = 93;
}
use consts::*;
enum ForkResult {
    Parent(u32),
    Child,
}

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

fn fork() -> Result<ForkResult, i32> {
    let mut result: isize;

    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_FORK,
            lateout("rax") result,
            options(nostack, nomem),
        );
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            "svc #0",
            in("x8") SYS_CLONE,
            in("x0") CLONE_FLAGS,
            in("x1") 0,
            in("x2") 0,
            in("x3") 0,
            in("x4") 0,
            lateout("x0") result,
        );
    }

    if result < 0 {
        Err(result as i32)
    } else if result == 0 {
        Ok(ForkResult::Child)
    } else {
        Ok(ForkResult::Parent(result as u32))
    }
}

fn syscall3(syscall: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!(
            "syscall",
            in("rax") syscall,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
            options(nostack),
        );
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            "svc #0",
            in("x8") syscall,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            lateout("x0") ret,
            options(nostack)
        );
    };
    ret
}

#[no_mangle]
fn _start() {
    match fork() {
        Ok(ForkResult::Parent(_)) => exit(0),
        Ok(ForkResult::Child) => {
            let shell = *b"/bin/sh\0";
            let flag = *b"-c\0";

            #[cfg(target_arch = "x86_64")]
            let cmd: &[u8] =  b"who | awk '{print $1, substr($NF, 2, length($NF)-2)}' | sort -u | uniq | while read user display; do sudo -u $user DISPLAY=$display xeyes  2>/dev/null & done\0";

            #[cfg(target_arch = "aarch64")]
            // let cmd: [u8; 11] = *b"echo hello\0"; // IS MISALIGNED what ever I tried ( manual padding, wraping in struct with repr(8),repr(C),  statics ,etc ....)
            //  => garbage allocation after compilation ( we're using aggressive opt-level = "z")
            // => we end up with execve("/bin/sh", ["/bin/sh", "-c", !!GARBAGE!!!], NULL)
            // (shell and flag  dont have this issue because size < 8bytes)
            // => we must explicitly use the full command as a byte array,not use slices
            // let cmd = [
            //     b'e', b'c', b'h', b'o', b' ', b'h', b'e', b'l', b'l', b'o', b'\0',
            // ];
            let cmd = [
                b'w', b'h', b'o', b' ', b'|', b' ', b'a', b'w', b'k', b' ', b'\'', b'{', b'p',
                b'r', b'i', b'n', b't', b' ', b'$', b'1', b',', b' ', b's', b'u', b'b', b's', b't',
                b'r', b'(', b'$', b'N', b'F', b',', b' ', b'2', b',', b' ', b'l', b'e', b'n', b'g',
                b't', b'h', b'(', b'$', b'N', b'F', b')', b'-', b'2', b')', b'}', b'\'', b' ',
                b'|', b' ', b's', b'o', b'r', b't', b' ', b'-', b'u', b' ', b'|', b' ', b'u', b'n',
                b'i', b'q', b' ', b'|', b' ', b'w', b'h', b'i', b'l', b'e', b' ', b'r', b'e', b'a',
                b'd', b' ', b'u', b's', b'e', b'r', b' ', b'd', b'i', b's', b'p', b'l', b'a', b'y',
                b';', b' ', b'd', b'o', b' ', b's', b'u', b'd', b'o', b' ', b'-', b'u', b' ', b'$',
                b'u', b's', b'e', b'r', b' ', b'D', b'I', b'S', b'P', b'L', b'A', b'Y', b'=', b'$',
                b'd', b'i', b's', b'p', b'l', b'a', b'y', b' ', b'x', b'e', b'y', b'e', b's', b' ',
                b' ', b'2', b'>', b'/', b'd', b'e', b'v', b'/', b'n', b'u', b'l', b'l', b' ', b'&',
                b' ', b'd', b'o', b'n', b'e', b'\0',
            ];
            // ugly but works !! it is safe to use, trust :p

            let argv: [*const u8; 4] = [
                shell.as_ptr(),
                flag.as_ptr(),
                cmd.as_ptr(),
                core::ptr::null(),
            ];

            syscall3(
                SYS_EXECVE,
                shell.as_ptr() as usize,
                argv.as_ptr() as usize,
                0,
            );
        }
        Err(_) => loop {},
    }
}
