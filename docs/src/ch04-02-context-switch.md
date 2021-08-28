# 上下文切换

> 本节介绍 trapframe-rs 中 [fncall.rs] 的魔法实现

[fncall.rs]: https://github.com/rcore-os/trapframe-rs/blob/master/src/arch/x86_64/fncall.rs

## 保存和恢复通用寄存器

> 定义 UserContext 结构体

```rust
pub struct UserContext {
    pub general: GeneralRegs,
    pub trap_num: usize,
    pub error_code: usize,
}
```
```rust
pub struct GeneralRegs {
    pub rax: usize,
    pub rbx: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rbp: usize,
    pub rsp: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    pub rip: usize,
    pub rflags: usize,
    pub fsbase: usize,
    pub gsbase: usize,
}
```
`Usercontext`保存了用户执行的上下文，包括跳转到用户态之后程序的第一条指令的地址，如果程序首次从内核态进入用户态执行，则rip指向用户进程的第一条指令的地址。
> 保存 callee-saved 寄存器到栈上，恢复 UserContext 寄存器，进入用户态，反之亦然
```rust
syscall_fn_return:
    # save callee-saved registers
    push r15
    push r14
    push r13
    push r12
    push rbp
    push rbx

    push rdi
    SAVE_KERNEL_STACK
    mov rsp, rdi

    POP_USER_FSBASE

    # pop trap frame (struct GeneralRegs)
    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop rbp
    pop r8                  # skip rsp
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
    pop r11                 # r11 = rip. FIXME: don't overwrite r11!
    popfq                   # pop rflags
    mov rsp, [rsp - 8*11]   # restore rsp
    jmp r11                 # restore rip
```
弹出的寄存器恰好对应了GeneralRegs的结构，通过在rust的unsafe代码块中调用`syscall_fn_return`函数，并且传递`Usercontext`结构体的指针到rdi中，可以创造出程序进入用户态的运行环境。


## 找回内核上下文：线程局部存储 与 FS 寄存器

> 在用户程序跳转回内核代码的那一刻，如何在不破坏用户寄存器的情况下切换回内核栈？
>
> 进入用户态前，将内核栈指针保存在内核 glibc 的 TLS 区域中。为此我们需要查看 glibc 源码，找到一个空闲位置。
>
> Linux 和 macOS 下如何分别通过系统调用设置 fsbase / gsbase

## 测试

> 编写单元测试验证上述过程

```rust
#[cfg(test)]
mod tests {
    use crate::*;

    #[cfg(target_os = "macos")]
    global_asm!(".set _dump_registers, dump_registers");

    // Mock user program to dump registers at stack.
    global_asm!(
        r#"
dump_registers:
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rsp
    push rbp
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax

    add rax, 10
    add rbx, 10
    add rcx, 10
    add rdx, 10
    add rsi, 10
    add rdi, 10
    add rbp, 10
    add r8, 10
    add r9, 10
    add r10, 10
    add r11, 10
    add r12, 10
    add r13, 10
    add r14, 10
    add r15, 10

    call syscall_fn_entry
"#
    );

    #[test]
    fn run_fncall() {
        extern "sysv64" {
            fn dump_registers();
        }
        let mut stack = [0u8; 0x1000];
        let mut cx = UserContext {
            general: GeneralRegs {
                rax: 0,
                rbx: 1,
                rcx: 2,
                rdx: 3,
                rsi: 4,
                rdi: 5,
                rbp: 6,
                rsp: stack.as_mut_ptr() as usize + 0x1000,
                r8: 8,
                r9: 9,
                r10: 10,
                r11: 11,
                r12: 12,
                r13: 13,
                r14: 14,
                r15: 15,
                rip: dump_registers as usize,
                rflags: 0,
                fsbase: 0, // don't set to non-zero garbage value
                gsbase: 0,
            },
            trap_num: 0,
            error_code: 0,
        };
        cx.run_fncall();
        // check restored registers
        let general = unsafe { *(cx.general.rsp as *const GeneralRegs) };
        assert_eq!(
            general,
            GeneralRegs {
                rax: 0,
                rbx: 1,
                rcx: 2,
                rdx: 3,
                rsi: 4,
                rdi: 5,
                rbp: 6,
                // skip rsp
                r8: 8,
                r9: 9,
                r10: 10,
                // skip r11
                r12: 12,
                r13: 13,
                r14: 14,
                r15: 15,
                ..general
            }
        );
        // check saved registers
        assert_eq!(
            cx.general,
            GeneralRegs {
                rax: 10,
                rbx: 11,
                rcx: 12,
                rdx: 13,
                rsi: 14,
                rdi: 15,
                rbp: 16,
                // skip rsp
                r8: 18,
                r9: 19,
                r10: 20,
                // skip r11
                r12: 22,
                r13: 23,
                r14: 24,
                r15: 25,
                ..cx.general
            }
        );
        assert_eq!(cx.trap_num, 0x100);
        assert_eq!(cx.error_code, 0);
    }
}
```

## macOS 的麻烦：动态二进制修改

> 由于 macOS 用户程序无法修改 fs 寄存器，当运行相关指令时会访问非法内存地址触发段错误。
> 
> 我们需要实现段错误信号处理函数，并在其中动态修改用户程序指令，将 fs 改为 gs。
