# 物理内存：按页分配的 VMO

## 简介

> 说明一下：Zircon 的官方实现中为了高效支持写时复制，使用了复杂精巧的树状数据结构，但它同时也引入了复杂性和各种 Bug。
> 我们在这里只实现一个简单版本，完整实现留给读者自行探索。
>
> 介绍 commit 操作的意义和作用

## HAL：物理内存管理

> 在 HAL 中实现 PhysFrame 和最简单的分配器

## 辅助结构：BlockRange 迭代器

> 实现 BlockRange

## 实现按页分配的 VMO

> 实现 for_each_page, commit, read, write 函数

## VMO 复制

> 实现 create_child 函数
