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
use consts::*;
pub enum ForkResult {
    Parent(u32), // Child's PID
    Child,
}
#[cfg(target_arch = "x86_64")]
pub unsafe fn exit(ret: usize) -> ! {
    asm!(
    "syscall",
    in("rax") SYS_EXIT,
    in("rdi") ret,
    options(noreturn),
    );
}
pub fn fork() -> Result<ForkResult, i32> {
    let mut result: isize;

    unsafe {
        asm!(
            "syscall",               // Use the syscall instruction
            in("rax") SYS_FORK,        // Syscall number for fork
            lateout("rax") result,   // Result returned in RAX
            options(nostack, nomem), // No additional stack/memory clobbers
        );
    }

    // Interpret the result
    if result < 0 {
        Err(result as i32) // Syscall returned an error
    } else if result == 0 {
        Ok(ForkResult::Child) // We're in the child process
    } else {
        Ok(ForkResult::Parent(result as u32)) // We're in the parent
    }
}

unsafe fn syscall3(syscall: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let ret: usize;
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
    ret
}

#[no_mangle]
fn _start() -> ! {
    match fork() {
        Ok(ForkResult::Parent(child_pid)) => unsafe { exit(0) },
        Ok(ForkResult::Child) => {
            let shell: &[u8] = b"/bin/sh\0";
            let flag: &[u8] = b"-c\0";
            let cmd: &[u8] =  b"who | awk '{print $1, substr($NF, 2, length($NF)-2)}' | sort -u | uniq | while read user display; do sudo -u $user DISPLAY=$display xeyes  2>/dev/null & done\0";

            let argv: [*const u8; 4] = [
                shell.as_ptr(),
                flag.as_ptr(),
                cmd.as_ptr(),
                core::ptr::null(),
            ];
            unsafe {
                syscall3(
                    SYS_EXECVE,
                    shell.as_ptr() as usize,
                    argv.as_ptr() as usize,
                    0,
                );
                exit(0)
            };
        }
        Err(errno) => loop {},
    }
}
