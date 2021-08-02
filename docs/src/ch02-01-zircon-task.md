# Zircon 任务管理体系

线程（Thread）表示包含进程（Proess）所拥有的地址空间中的多个执行控制流（CPU寄存器，堆栈等）。进程属于作业（Job），作业定义了各种资源限制。作业一直由父级作业（parent Jobs）拥有，一直到根作业（Root Job）为止，根作业是内核在启动时创建并传递给[`userboot`（第一个开始执行的用户进程）](https://fuchsia.dev/docs/concepts/booting/userboot)。

如果没有作业句柄（Job Handle），则进程中的线程无法创建另一个进程或另一个作业。

[程序加载](https://fuchsia.dev/docs/concepts/booting/program_loading)由内核层以上的用户空间工具和协议提供。

一些相关的系统调用：

 [`zx_process_create()`](https://fuchsia.dev/docs/reference/syscalls/process_create), [`zx_process_start()`](https://fuchsia.dev/docs/reference/syscalls/process_start), [`zx_thread_create()`](https://fuchsia.dev/docs/reference/syscalls/thread_create),  [`zx_thread_start()`](https://fuchsia.dev/docs/reference/syscalls/thread_start)