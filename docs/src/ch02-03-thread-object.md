# 线程管理：Thread 对象

线程对象是代表分时CPU的执行上下文的一种结构。线程对象与特定的进程对象相关联，该进程对象为线程对象执行中涉及的I/O和计算提供提供必要的内存和其他对象的句柄。


## 生命期

线程是通过调用Thread::create()创建的，但只有在调用Thread::create()或Process::start()时才开始执行。这两个系统调用的参数都是要执行的初始例程的入口。

传递给Process::start()的线程应该是在一个进程上开始执行的第一个线程。


下列情况都可导致一个线程终止执行：

- 通过调用 `CurrentThread::exit()`
- 当父进程终止时
- 通过调用 `Task::kill()`
- 在生成没有处理程序或处理程序决定终止线程的异常之后。

从入口例程返回并不终止执行。入口点的最后一个动作应该是调用CurrentThread::exit()。



关闭一个线程的最后一个句柄并不终止执行。为了强行杀死一个没有可用句柄的线程，可以使用KernelObject::get_child()来获得该线程的句柄。但这种方法是非常不可取的。杀死一个正在执行的线程可能会使进程处于损坏的状态。

本地线程总是分离的（*detached*）。也就是说，不需要join()操作来做一个干净的终止（clean termination）。但一些内核之上的运行系统，如C11或POSIX可能需要线程被连接（be joined）。



## 信号

线程提供以下信号：

- THREAD_TERMINATED
- THREAD_SUSPENDED
- THREAD_RUNNING

当一个线程启动执行时，THREAD_RUNNING被设定。当它被暂停时，THREAD_RUNNING被取消，THREAD_SUSPENDED被设定。当线程恢复时，THREAD_SUSPENDED被取消，THREAD_RUNNING被设定。当线程终止时，THREAD_RUNNING和THREAD_SUSPENDED都被置位，THREAD_TERMINATED也被置位。

注意，信号经过“或”运算后进入KernelObject::wait_signal()函数系列所保持的状态 ，因此当它们返回时，你可能会看到所要求的信号的任何组合。



## 线程状态(ThreadState)

> 状态转移：创建 -> 运行 -> 暂停 -> 退出，最好有个状态机的图
>
> 实现 ThreadState，最好能加一个单元测试来验证转移过程

```rust
pub enum ThreadState {
    New,                  \\该线程已经创建，但还没有开始运行
    Running,              \\该线程正在正常运行用户代码
    Suspended,            \\由于zx_task_suspend()而暂停
    Blocked,              \\在一个系统调用中或处理一个异常而阻塞
    Dying,                \\线程正在被终止的过程中，但还没有停止运行
    Dead,                 \\该线程已停止运行
    BlockedException,     \\该线程在一个异常中被阻塞
    BlockedSleeping,      \\该线程在zx_nanosleep()中被阻塞
    BlockedFutex,         \\该线程在zx_futex_wait()中被阻塞
    BlockedPort,          \\该线程在zx_port_wait()中被被阻塞
    BlockedChannel,       \\该线程在zx_channel_call()中被阻塞
    BlockedWaitOne,       \\该线程在zx_object_wait_one()中被阻塞 
    BlockedWaitMany,      \\该线程在zx_object_wait_many()中被阻塞
    BlockedInterrupt,     \\该线程在zx_interrupt_wait()中被阻塞
    BlockedPager,         \\被Pager阻塞 （目前没用到???）
}
```



## 线程寄存器上下文

> 定义 ThreadState，实现 read_state，write_state



## Async 运行时和 HAL 硬件抽象层

> 简单介绍 async-std 的异步机制
>
> 介绍 HAL 的实现方法：弱链接
>
> 实现 hal_thread_spawn

## 线程启动

> 将 HAL 接入 Thread::start，编写单元测试验证能启动多线程
