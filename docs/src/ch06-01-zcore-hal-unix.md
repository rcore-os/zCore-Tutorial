# zCore 的用户态运行支持

libos 版 zCore（简称uzCore） 的开发与裸机版 zCore （简称bzCore）同步进行,两个版本的 zCore 共用除了HAL 层之外的所有代码。为了支持 uzCore  的正常运行,zCore 在地址空间划分方面对 Zircon /Linux的原有设计进行了一定的修改,并为此对 Fuchsia 的源码进行了简单的修改、重新编译;另外,uzCore 需要的硬件相关层(HAL)将完全由宿主 OS 提供支持,一个合理的 HAL 层接口划分也是为支持 uzCore做出的重要考虑。

## HAL 层接口设计

HAL 层的设计是在bzCore 和 uzCore 的开发过程中逐渐演进形成的，在开发过程中将硬件实现相关的接口,比如页表、物理内存分配等进行封装,暴露给上层的内核对象层使用。在 kernel­-hal 模块中,给出空的弱链接实现,由 bzCore 或 uzCore 的开发者对相应的接口进行相应的实现,并用设定函数链接名称的方式,替换掉预设的弱链接的空函数。在整个开发过程中,不断对 HAL 层提出需求并实现,目前形成了第一版 HAL 层接口,在设计上能够满足现有的内核对象实现所需要的功能。

对内核对象层而言,所依赖的硬件环境不再是真实硬件环境中能够看到的物理内存、CPU、MMU 等,而是 HAL 暴露给上层的一整套接口。这一点从设计上来说,是 zCore 与 Zircon 存在差异的一点。Zircon 将 x86_64 、ARM64 的硬件架构进行底层封装,但是没有给出一套统一的硬件 API 供上层的内核对象直接使用,在部分内核对象的实现中,仍然需要通过宏等手段对代码进行条件编译,从而支持同时面向两套硬件架构进行开发。而在 zCore 的内核对象层实现中,可以完全不考虑底层硬件接口的实现,使一套内核对象的模块代码可以同时在 bzCore和 uzCore 上运行,之后如果 zCore 进一步支持 RISC-V 64 架构（已初步实现）,只需要新增一套 HAL的实现,无需修改上层代码。下面将列出目前的uzCore的HAL层，即kernel-hal-unix的接口。


### **HAL接口名称    功能描述**

* 线程相关
    * hal_thread_spawn  Thread::spawn创建一个新线程并加入调度
    * hal_thread_set_tid Thread::set_tid  设定当前线程的 id
    * hal_thread_get_tid Thread::get_tid  获取当前线程的 id
* future
    * yield_now暂时让出 CPU，回到async runtime中
    * sleep_until 休眠直到定时到达
    * YieldFuture 放弃执行的future
    * SleepFuture 睡眠且等待被唤醒的future
    * SerialFuture 通过serial_read获得字符的future
* 上下文切换相关
    * VectorRegs  x86相关
    * hal_context_run context_run 进入“用户态”运行
* 用户指针相关
    * UserPtr  对用户指针的操作：读/写/解引用/访问数组/访问字符串
    * IoVec 非连续buffer集合（Vec结构）：读/写
* 页表相关
    * hal_pt_currentPageTable::current  获取当前页表
    * hal_pt_newPageTable::new  新建一个页表
    * hal_pt_map PageTable::map  将一个物理页帧映射到一个虚拟地址中
    * hal_pt_unmap PageTable::unmap  解映射某个虚拟地址
    * hal_pt_protect PageTable::protect 修改vaddr对应的页表项的flags
    * hal_pt_query PageTable::query  查询某个虚拟地址对应的页表项状态
    * hal_pt_table_phys PageTable::table_phys  获取对应页表的根目录表物理地址
    * hal_pt_activate PageTable::activate 激活当前页表
    * PageTable::map_many  同时映射多个物理页帧到连续虚拟内存空间
    * PageTable::map_cont  同时映射连续的多个物理页帧到虚拟内存空间
    * hal_pt_unmap_cont PageTable::unmap_cont  解映射某个虚拟地址开始的一片范围
    * MMUFlags  页表项的属性位
* 物理页帧相关
    * hal_frame_alloc PhysFrame::alloc  分配一个物理页帧
    * hal_frame_alloc_contiguous  PhysFrame::alloc_contiguous_base  分配一块连续的物理内存
    * PhysFrame::addr  返回物理页帧对应的物理地址
    * PhysFrame::alloc_contiguous  分配一块连续的物理内存
    * PhysFrame::zero_frame_addr  返回零页的物理地址（一个特殊页，内容永远为全0）
    * PhysFrame::drop  Drop trait 回收该物理页帧
    * hal_pmem_read pmem_read  读取某特定物理页帧的内容到缓冲区
    * hal_pmem_write pmem_write  将缓冲区中的内容写入某特定物理页帧
    * hal_frame_copy frame_copy  复制物理页帧的内容
    * hal_frame_zero  frame_zero_in_range  物理页帧清零
    * hal_frame_flush  frame_flush将物理页帧的数据从 Cache 刷回内存
* 基本I/O外设
    * hal_serial_read  serial_read  字符串输入
    * hal_serial_write  serial_write  字符串输出
    * hal_timer_now  timer_now 获取当前时间
    * hal_timer_set  timer_set 设置一个时钟，当到达deadline时，会调用 callback 函数
    * hal_timer_set_next  timer_set_next  设置下一个时钟
    * hal_timer_tick  timer_tick当时钟中断产生时会调用的时钟函数，触发所有已到时间的 callback
* 中断处理
    * hal_irq_handle  handle 中断处理例程
    * hal_ioapic_set_handle set_ioapic_handle x86相关，对高级中断控制器设置处理例程
    * hal_irq_add_handle  add_handle 对某中断添加中断处理例程
    * hal_ioapic_reset_handle reset_ioapic_handle 重置级中断控制器并设置处理例程
    * hal_irq_remove_handle  remove_handle 移除某中断的中断处理例程
    * hal_irq_allocate_block  allocate_block 给某中断分配连续区域
    * hal_irq_free_block  free_block 给某中断释放连续区域
    * hal_irq_overwrite_handler  overwrite_handler 覆盖某中断的中断处理例程
    * hal_irq_enable  enable  使能某中断
    * hal_irq_disable disable  屏蔽某中断
    * hal_irq_maxinstr maxinstr  x86相关，获得IOAPIC的maxinstr???
    * hal_irq_configure  configure  对某中断进行配置???
    * hal_irq_isvalid  is_valid 查询某中断是否有效
* 硬件平台相关
    * hal_vdso_constants  vdso_constants  得到平台相关常量参数
        * struct VdsoConstants  平台相关常量：

max_num_cpus features dcache_line_size ticks_per_second  ticks_to_mono_numerator ticks_to_mono_denominator physmem version_string_len  version_string

    * fetch_fault_vaddr  fetch_fault_vaddr 取得出错的地址 ???好像缺了hal_*
    * fetch_trap_num fetch_trap_num 取得中断号
    * hal_pc_firmware_tables  pc_firmware_tables  x86相关，取得`acpi_rsdp` 和 `smbios` 的物理地址
    * hal_acpi_table get_acpi_table 得到acpi table
    * hal_outpd  outpd  x86相关，对IO Port进行写访问
    * hal_inpd  inpd  x86相关，对IO Port进行读访问
    * hal_apic_local_id  apic_local_id 得到本地(local) APIC  ID
    * fill_random 产生随机数，并写入到buffer中

在上述“线程相关”的列表中，列出了 HAL 层的部分接口设计,覆盖线程调度方面。在线程调度方面,Thread 结构体相关的接口主要用于将一个线程加入调度等基本操作。在 zCore 的相关实现中,线程调度的各接口使用 naive­-executor 给出的接口以及 trapframe­ 给出的接口来进行实现,二者都是我们为裸机环境的协程调度与上下文切换所封装的 Rust 库。uzCore 中,线程调度的相关接口依赖于 Rust 的用户态协程支持以及 uzCore 开发者实现的用户态上下文切换。

在内存管理方面,HAL 层将内存管理分为页表操作与物理页帧管理两方面,并以此设计接口。在 zCore 实现中,物理页帧的分配与回收由于需要设计物理页帧分配器,且可分配范围大小与内核刚启动时的内存探测密切相关,我们将其直接在总控模块 zCore 中进行实现。而在 uzCore 中,页表对应操作依赖 mmap 进行模拟,物理页帧的相关操作则直接使用用户态物理内存分配器进行模拟。

在 Zircon 的设计中,内存的初始状态应该设置为全 0,为了在内核对象层满足该要求,我们为 HAL 层设计了零页接口,要求 HAL 层保留一个内容为全 0 的物理页帧,供上层使用。上层负责保证该零页内容不被修改。


## 修改 VDSO

VDSO 是由内核提供、并只读映射到用户态的动态链接库，以函数接口形式提供系统调用接口。原始的 VDSO 中将会最终使用 syscall 指令从用户态进入内核态。但在 uzCore 环境下,内核和用户程序都运行在用户态,因此需要将 syscall 指令修改为函数调用,也就是将 sysall 指令修改为 call 指令。为此我们修改了 VDSO 汇编代码，将其中的 syscall 替换为 call，提供给 uzCore 使用。在 uzCore 内核初始化环节中，向其中填入 call 指令要跳转的目标地址,重定向到内核中处理 syscall 的特定函数,从而实现模拟系统调用的效果。





## 调整地址空间范围

在 uzCore 中,使用 mmap 来模拟页表,所有进程共用一个 64 位地址空间。因此,从地址空间范围这一角度来说,运行在 uzCore 上的用户程序所在的用户进程地址空间无法像 Zircon 要求的一样大。对于这一点,我们在为每一个用户进程设置地址空间时,手动进行分配,规定每一个用户进程地址空间的大小为 0x100_0000_0000,从 0x2_0000_0000 开始依次排布。0x0 开始至 0x2_0000_0000 规定为 uzCore 内核所在地址空间,不用于 mmap。图 3.3给出了 uzCore 在运行时若干个用户进程的地址空间分布。

与 uzCore 兼容,zCore 对于用户进程的地址空间划分也遵循同样的设计,但在裸机环境下,一定程度上摆脱了限制,能够将不同用户地址空间分隔在不同的页表中。如图 3.4所示,zCore 中将三个用户进程的地址空间在不同的页表中映射,但是为了兼容 uzCore 的运行,每一个用户进程地址空间中用户程序能够真正访问到的部分都仅有 0x100_0000_0000 大小。


## LibOS源代码分析记录

### zCore on riscv64的LibOS支持

* LibOS unix模式的入口在linux-loader main.rs:main()

初始化包括kernel_hal_unix，Host文件系统，其中载入elf应用程序的过程与zcore bare模式一样；

重点工作应该在kernel_hal_unix中的**内核态与用户态互相切换**的处理。

kernel_hal_unix初始化主要包括了，构建Segmentation Fault时SIGSEGV信号的处理函数，当代码尝试使用fs寄存器时会触发信号；

* 为什么要注册这个信号处理函数呢？

根据wrj的说明：由于 macOS 用户程序无法修改 fs 寄存器，当运行相关指令时会访问非法内存地址触发Segmentation Fault。故实现段错误信号处理函数，并在其中动态修改用户程序指令，将 fs 改为 gs

kernel_hal_unix还构造了**进入用户态**所需的run_fncall() -> syscall_fn_return()；

而用户程序需要调用syscall_fn_entry()来**返回内核态**；

Linux-x86_64平台运行时，用户态和内核态之间的切换运用了 fs base 寄存器；

* Linux 和 macOS 下如何分别通过系统调用设置 fsbase / gsbase 。

这个转换过程调用到了trapframe库，x86_64和aarch64有对应实现，而riscv则需要自己手动实现；

* 关于fs寄存器

查找了下，fs寄存器一般会用于寻址TLS，每个线程有它自己的fs base地址；

fs寄存器被glibc定义为存放tls信息，结构体tcbhead_t就是用来描述tls；

进入用户态前，将内核栈指针保存在内核 glibc 的 TLS 区域中。

可参考一个运行时程序的代码转换工具：[https://github.com/DynamoRIO/dynamorio/issues/1568#issuecomment-239819506](https://github.com/DynamoRIO/dynamorio/issues/1568#issuecomment-239819506?fileGuid=VMAPV7ERl7HbpNqg)

* **LibOS内核态与用户态的切换**

Linux x86_64中，fs寄存器是用户态程序无法设置的，只能通过系统调用进行设置；

例如clone系统调用，通过arch_prctl来设置fs寄存器；指向的struct pthread，glibc中，其中的首个结构是tcbhead_t

计算tls结构体偏移：

经过试验，x86_64平台，int型：4节，指针类型：8节，无符号长整型：8节；

riscv64平台，int型： 4节，指针类型：8节，无符号长整型：8节；

计算tls偏移量时，注意下，在musl中，aarch64和riscv64架构有#define TLS_ABOVE_TP，而x86_64无此定义

* 关于Linux user mode (UML)

"No, UML works only on x86 and x86_64."

[https://sourceforge.net/p/user-mode-linux/mailman/message/32782012/](https://sourceforge.net/p/user-mode-linux/mailman/message/32782012/?fileGuid=VMAPV7ERl7HbpNqg)

