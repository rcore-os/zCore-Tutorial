# 虚拟内存：VMAR 对象

## VMAR 简介

虚拟内存地址区域（Virtual Memory Address Regions ，VMARs）为管理进程的地址空间提供了一种抽象。在进程创建时，将Root VMAR 的句柄提供给进程创建者。该句柄指的是跨越整个地址空间的 VMAR。这个空间可以通过[`zx_vmar_map()`](https://fuchsia.dev/docs/reference/syscalls/vmar_map)和 [`zx_vmar_allocate()`](https://fuchsia.dev/docs/reference/syscalls/vmar_allocate)接口来划分 。 [`zx_vmar_allocate()`](https://fuchsia.dev/docs/reference/syscalls/vmar_allocate)可用于生成新的 VMAR（称为子区域或子区域），可用于将地址空间的各个部分组合在一起。

## 实现 VMAR 对象框架

> 定义 VmAddressRange，VmMapping
>
> 实现 create_child, map, unmap, destroy 函数，并做单元测试验证地址空间分配

## HAL：用 mmap 模拟页表

> 实现页表接口 map, unmap, protect

## 实现内存映射

> 用 HAL 实现上面 VMAR 留空的部分，并做单元测试验证内存映射
