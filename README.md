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
