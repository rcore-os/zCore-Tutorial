# 等待内核对象的信号

## 信号与等待机制简介

## 在内核对象中加入信号

> 定义 Signal 结构体
>
> 在 KObjectBase 中加入 signal 和 callbacks 变量，实现 signal 系列函数，并做单元测试

## 实现信号等待 Future

> 实现 wait_signal 函数，并做单元测试

## 利用 select 组合子实现多对象等待

> 实现 wait_signal_many 函数，并做单元测试
