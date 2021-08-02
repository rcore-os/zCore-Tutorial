# 物理内存：VMO 对象

## VMO 简介

> 根据文档梳理 VMO 的主要特性

虚拟拟内存对象（Virtual Memory Objects， VMO）代表一组物理内存页面，或 潜在的页面（将根据需要延迟创建/填充）。

它们可以通过 [`zx_vmar_map()`](https://fuchsia.dev/docs/reference/syscalls/vmar_map)被映射到一个进程（Process）的地址空间，也可通过 [`zx_vmar_unmap()`](https://fuchsia.dev/docs/reference/syscalls/vmar_unmap)来解除映射。可以使用[`zx_vmar_protect()`](https://fuchsia.dev/docs/reference/syscalls/vmar_protect)来调整映射页面的权限。

也可以直接使用[`zx_vmo_read()`](https://fuchsia.dev/docs/reference/syscalls/vmo_read)来读取VMO和通过使用 [`zx_vmo_write()`](https://fuchsia.dev/docs/reference/syscalls/vmo_write)来写入 VMO。因此，通过诸如“创建 VMO，将数据集写入其中，然后将其交给另一个进程使用”等一次性（one-shot ）操作，可以避免将它们映射到地址空间的开销。

## 实现 VMO 对象框架

> 实现 VmObject 结构，其中定义 VmObjectTrait 接口，并提供三个具体实现 Paged, Physical, Slice

## HAL：用文件模拟物理内存

> 初步介绍 mmap，引出用文件模拟物理内存的思想
>
> 创建文件并用 mmap 线性映射到进程地址空间
>
> 实现 pmem_read, pmem_write

## 实现物理内存 VMO

> 用 HAL 实现 VmObjectPhysical 的方法，并做单元测试

## 实现切片 VMO

> 实现 VmObjectSlice，并做单元测试
