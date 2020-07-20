# 初识内核对象

## 内核对象简介

TODO

详细内容可参考 Fuchsia 官方文档：[内核对象]，[句柄]，[权限]。

在这一节中我们将在 Rust 中实现以上三个概念。

[内核对象]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/objects.md
[句柄]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/handles.md
[权限]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/rights.md

## 建立项目

首先我们需要安装 Rust 工具链。在 Linux 或 macOS 系统下，只需要用一个命令下载安装 rustup 即可：

```sh
$ curl https://sh.rustup.rs -sSf | sh
```

具体安装方法可以参考[官方文档]。

[官方文档]: https://kaisery.github.io/trpl-zh-cn/ch01-01-installation.html

接下来我们用 cargo 创建一个 Rust 库项目：

```sh
$ cargo new --lib zcore
$ cd zcore
```

我们将在这个 crate 中实现所有的内核对象，以库（lib）而不是可执行文件（bin）的形式组织代码，后面我们会依赖单元测试保证代码的正确性。

由于我们会用到一些不稳定（unstable）的语言特性，需要使用 nightly 版本的工具链。在项目根目录下创建一个 `rust-toolchain` 文件，指明使用的工具链版本：

```sh
{{#include ../../zcore/rust-toolchain}}
```

这个程序库目前是在你的 Linux 或 macOS 上运行，但有朝一日它会成为一个真正的 OS 在裸机上运行。
为此我们需要移除对标准库的依赖，使其成为一个不依赖当前 OS 功能的库。在 `lib.rs` 的第一行添加一个声明：

```rust,no_run,noplaypen
#![no_std]
```

现在我们可以尝试运行一下自带的单元测试，编译器可能会自动下载并安装工具链：

```sh
$ cargo test
...
    Finished test [unoptimized + debuginfo] target(s) in 0.52s
     Running target/debug/deps/zcore-dc6d43637bc5df7a

running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 实现 KernelObject 接口

## 句柄 Handle

## 权限 Rights