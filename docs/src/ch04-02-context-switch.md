# 上下文切换

> 本节介绍 trapframe-rs 中 [fncall.rs] 的魔法实现

[fncall.rs]: https://github.com/rcore-os/trapframe-rs/blob/master/src/arch/x86_64/fncall.rs

## 保存和恢复通用寄存器

> 定义 UserContext 结构体
>
> 保存 callee-saved 寄存器到栈上，恢复 UserContext 寄存器，进入用户态，反之亦然

## 找回内核上下文：线程局部存储 与 FS 寄存器

> 在用户程序跳转回内核代码的那一刻，如何在不破坏用户寄存器的情况下切换回内核栈？
>
> 进入用户态前，将内核栈指针保存在内核 glibc 的 TLS 区域中。为此我们需要查看 glibc 源码，找到一个空闲位置。
>
> Linux 和 macOS 下如何分别通过系统调用设置 fsbase / gsbase

## 测试

> 编写单元测试验证上述过程

## macOS 的麻烦：动态二进制修改

> 由于 macOS 用户程序无法修改 fs 寄存器，当运行相关指令时会访问非法内存地址触发段错误。
> 
> 我们需要实现段错误信号处理函数，并在其中动态修改用户程序指令，将 fs 改为 gs。
