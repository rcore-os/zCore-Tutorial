# 同时等待多个信号：Port 对象

## Port 对象简介

> 同时提及一下 Linux 的 epoll 机制作为对比

## 实现 Port 对象框架

> 定义 Port 和 PortPacket 结构体

## 实现事件推送和等待

> 实现 KernelObject::send_signal_to_port 和 Port::wait 函数，并做单元测试
