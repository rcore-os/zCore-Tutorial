# Zircon 用户程序

## 用户态启动流程

> kernel -> userboot -> bootsvc -> component_manager -> sh / device_manager
>
> ZBI 与 bootfs：ZBI 中包含初始文件系统 bootfs，内核将 ZBI 完整传递给 userboot，由它负责解析并对其它进程提供文件服务

## 用户程序的组成

> 内核不直接参与用户程序的加载工作（第一个进程除外）
>
> 用户程序强制使用 PIC 和 PIE（位置无关代码）
>
> 内存地址空间组成：Program, Stack, vDSO, Dylibs
>
> 通过 Channel 传递启动信息和句柄

## 加载 ELF 文件

> 简单介绍 ELF 文件的组成结构
>
> 实现 VmarExt::load_from_elf 函数

## 系统调用的跳板：vDSO

> 介绍 vDSO 的作用
>
> 如何修改 vDSO 源码（libzircon）将 syscall 改为函数调用
>
> 加载 vDSO 时修改 vDSO 代码段，填入跳转地址

## 第一个用户程序：userboot

> 实现 zircon-loader 中的 run_userboot 函数
> 
> 能够进入用户态并在第一个系统调用时跳转回来
