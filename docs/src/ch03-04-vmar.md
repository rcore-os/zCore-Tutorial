# 虚拟内存：VMAR 对象

## VMAR 简介

## 实现 VMAR 对象框架

> 定义 VmAddressRange，VmMapping
>
> 实现 create_child, map, unmap, destroy 函数，并做单元测试验证地址空间分配

## HAL：用 mmap 模拟页表

> 实现页表接口 map, unmap, protect

## 实现内存映射

> 用 HAL 实现上面 VMAR 留空的部分，并做单元测试验证内存映射
