# 简明 zCore 教程

[简明 zCore 教程](README.md)
[zCore 整体结构和设计模式](zcore-intro.md)
[Fuchsia OS 和 Zircon 微内核](fuchsia.md)

- [内核对象](ch01-00-object.md)
    - [初识内核对象](ch01-01-kernel-object.md)
    - [对象管理器：Process 对象](ch01-02-process-object.md)
    - [对象传送器：Channel 对象](ch01-03-channel-object.md)

- [任务管理](ch02-00-task.md)
    - [Zircon 任务管理体系](ch02-01-zircon-task.md)
    - [硬件抽象层与异步运行时](ch02-02-hal-async.md)
    - [线程管理：Thread 对象](ch02-03-thread-object.md)
    - [进程管理：Process 与 Job 对象](ch02-04-process-job-object.md)

- [内存管理](ch03-00-memory.md)
    - [Zircon 内存管理模型](ch03-01-zircon-memory.md)
    - [物理内存：VMO 对象](ch03-02-vmo.md)
    - [虚拟内存：VMAR 对象](ch03-03-vmar.md)

- [用户程序](ch04-00-userspace.md)
    - [Zircon 用户程序](ch04-01-user-program.md)
    - [加载 ELF 文件](ch04-02-load-elf.md)
    - [上下文切换](ch04-03-context-switch.md)
    - [系统调用](ch04-04-syscall.md)
