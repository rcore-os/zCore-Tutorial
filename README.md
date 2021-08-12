# zCore Tutorial

[![CI](https://github.com/rcore-os/zCore-Tutorial/workflows/CI/badge.svg?branch=master)](https://github.com/rcore-os/zCore-Tutorial/actions)
[![Docs](https://img.shields.io/badge/docs-alpha-blue)](https://rcore-os.github.io/zCore-Tutorial/)

zCore Toturial 的目标是通过`step by step`地建立一个简化的zCore kernel的过程来学习和掌握zCore Kernel的核心概念和对应实现，从而为进一步分析掌握zCore的完整内核打下基础。

zCore Toturial 的特点是所有的code都在用户态运行，便于调试和分析。

## 仓库目录

* `docs/`: 教学实验指导
* `code`: 操作系统代码

## 实验指导

基于 mdBook，目前目前已经部署到了 [GitHub Pages](https://rcore-os.github.io/zCore-Tutorial/) 上面。

### 文档本地使用方法

```bash
git clone https://github.com/rcore-os/zCore-Tutorial.git
cd zCore-Tutorial
cargo install mdbook
mdbook serve docs
```

## code
`code`目录下的`rust-toolchain`内容为`nightly-2021-07-27`。原则上，我们会采用`rustc`最新的版本。目前的版本信息如下：
```
rustc 1.56.0-nightly (08095fc1f 2021-07-26)
```

## 学习顺序建议

### 初步了解

1. 阅读有关fuchsia/zircon的概述/简介文章，如 https://zh.wikipedia.org/zh-hans/Google_Fuchsia

2. 阅读 https://fuchsia.dev/fuchsia-src/concepts/kernel 了解zircon基本思想

3. 阅读潘庆霖毕设论文前两章，了解zCore的基本思想

### 逐渐深入
1. 阅读 https://fuchsia.dev/fuchsia-src/reference/syscalls 了解应用程序对Kernel的需求
2. 阅读 https://fuchsia.dev/fuchsia-src/reference/kernel_objects/objects 了解Kernel中各种object的含义和行为

### 理解设计实现

1. 阅读&分析本项目中的文档和代码，并对照上面的kernel概念，了解kernel概念和设计实现的对应关系

### 动手实践

1. 在分析和理解的基础上，改进本项目对应章节的文档

2. 在分析和理解的基础上，改进/优化本项目的代码，增加测试用例，增加功能

3. 在大致掌握本项目后，通过进一步理解和改进zCore，对zCore等新型操作系统有很好的感悟，提升自身实践能力

   

## 参考

- https://fuchsia.dev/
  - https://fuchsia.dev/fuchsia-src/concepts/kernel
  - https://fuchsia.dev/fuchsia-src/reference/kernel_objects/objects
  - https://fuchsia.dev/fuchsia-src/reference/syscalls
  - https://github.com/zhangpf/fuchsia-docs-zh-CN/tree/master/zircon
  - [许中兴博士演讲：Fuchsia OS 简介](https://xuzhongxing.github.io/201806fuchsia.pdf)
  
- 毕设论文
  - [Rust语言操作系统的设计与实现,王润基本科毕设论文,2019](https://github.com/rcore-os/zCore/wiki/files/wrj-thesis.pdf) 
  - [zCore操作系统内核的设计与实现,潘庆霖本科毕设论文,2020](https://github.com/rcore-os/zCore/wiki/files/pql-thesis.pdf)
  
- 开发文档
  - https://github.com/rcore-os/zCore/wiki/documents-of-zcore

- 更简单和基础的[rCore-Tutorial v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)：如果看不懂上面的内容，可以先看看这个教程。
