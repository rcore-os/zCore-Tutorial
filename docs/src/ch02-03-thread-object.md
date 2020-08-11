# 线程管理：Thread 对象

## 线程状态

> 状态转移：创建 -> 运行 -> 暂停 -> 退出，最好有个状态机的图
>
> 实现 ThreadState，最好能加一个单元测试来验证转移过程

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
