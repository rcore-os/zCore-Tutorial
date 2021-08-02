# 用户态同步互斥：Futex 对象

## Futex 机制简介

> Futex 是现代 OS 中用户态同步互斥的唯一底层设施
>
> 为什么快：利用共享内存中的原子变量，避免进入内核

Futexes 是内核原语，与用户空间原子操作一起使用以实现高效的同步原语（如Mutexes， Condition Variables等），它只需要在竞争情况（contended case）下才进行系统调用。通常它们实现在标准库中。

## 实现基础元语：wait 和 wake

> 实现 wait 和 wake 函数，并做单元测试

## 实现高级操作

> 实现 Zircon 中定义的复杂 API
