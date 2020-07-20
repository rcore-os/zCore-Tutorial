# 初识内核对象

## 内核对象简介

TODO

详细内容可参考 Fuchsia 官方文档：[内核对象]。

在这一节中我们将在 Rust 中实现内核对象的概念。

[内核对象]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/objects.md

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
为此我们需要移除对标准库的依赖，使其成为一个不依赖当前 OS 功能的库。在 `lib.rs` 的第一行添加声明：

```rust,noplaypen
#![no_std]
extern crate alloc;
```

现在我们可以尝试运行一下自带的单元测试，编译器可能会自动下载并安装工具链：

```sh
$ cargo test
    Finished test [unoptimized + debuginfo] target(s) in 0.52s
     Running target/debug/deps/zcore-dc6d43637bc5df7a

running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 实现 KernelObject 接口

所有的内核对象有一系列共同的属性和方法，我们称这些方法为对象的公共**接口（Interface）**。
同一种方法在不同类型的对象中可能会有不同的行为，在面向对象语言中我们称其为**多态（Polymorphism）**。

Rust 是一门部分面向对象的语言，我们通常用它的 trait 实现接口和多态。

首先创建一个 `KernelObject` trait 作为内核对象的公共接口：

```rust
/// 内核对象公共接口
pub trait KernelObject: Send + Sync {
{{#include ../../zcore/src/object/mod.rs:object}}

{{#include ../../zcore/src/object/mod.rs:koid}}
```

这里的 [`Send + Sync`] 是一个约束所有 `KernelObject` 都要满足的前提条件，即它必须是一个**并发对象**。
所谓并发对象指的是**可以安全地被多线程共享访问**。事实上我们的内核本身就是一个共享地址空间的多线程程序，在裸机上每个 CPU 核都可以被视为一个并发执行的线程。
由于内核对象可能被多个线程同时访问，因此它必须是并发对象。

[`Send + Sync`]: https://kaisery.github.io/trpl-zh-cn/ch16-04-extensible-concurrency-sync-and-send.html

## 实现一个空对象

接下来我们实现一个最简单的空对象 `DummyObject`，并为它实现 `KernelObject` 接口：

```rust,noplaypen
{{#include ../../zcore/src/object/mod.rs:dummy_def}}
```

这里我们采用一种[**内部可变性**]的设计模式：将对象的所有可变的部分封装到一个内部对象 `DummyObjectInner` 中，并在原对象中用自旋锁 [`Mutex`] 把它包起来，剩下的其它字段都是不可变的。
`Mutex` 会用最简单的方式帮我们处理好并发访问问题：如果有其他人正在访问，我就在这里死等。
数据被 `Mutex` 包起来之后需要首先使用 [`lock()`] 拿到锁之后才能访问。此时并发访问已经安全，因此被包起来的结构自动具有了 `Send + Sync` 特性。

[`Mutex`]: https://docs.rs/spin/0.5.2/spin/struct.Mutex.html
[`lock()`]: https://docs.rs/spin/0.5.2/spin/struct.Mutex.html#method.lock
[**内部可变性**]: https://kaisery.github.io/trpl-zh-cn/ch15-05-interior-mutability.html

使用自旋锁引入了新的依赖库 [`spin`] ，需要在 `Cargo.toml` 中加入以下声明：

[`spin`]: https://docs.rs/spin/0.5.2/spin/index.html

```toml
[dependencies]
{{#include ../../zcore/Cargo.toml:spin}}
```
 
然后我们为新对象实现构造函数：

```rust,noplaypen
{{#include ../../zcore/src/object/mod.rs:dummy_new}}
```

根据文档描述，每个内核对象都有唯一的 ID。为此我们需要实现一个全局的 ID 分配方法。这里采用的方法是用一个静态变量存放下一个待分配 ID 值，每次分配就原子地 +1。
ID 类型使用 `u64`，保证了数值空间足够大，在有生之年都不用担心溢出问题。在 Zircon 中 ID 从 1024 开始分配，1024 以下保留作内核内部使用。

另外注意这里 `new` 函数返回类型不是 `Self` 而是 `Arc<Self>`，这是为了以后方便而做的统一约定。

最后我们为它实现 `KernelObject` 接口：

```rust,noplaypen
{{#include ../../zcore/src/object/mod.rs:dummy_impl}}
```

到此为止，我们已经迈出了万里长征第一步，实现了一个最简单的功能。有实现，就要有测试！即使最简单的代码也要保证它的行为符合我们预期。
只有对现有代码进行充分测试，在未来做添加和修改的时候，我们才有信心不会把事情搞砸。俗话讲"万丈高楼平地起"，把地基打好才能盖摩天大楼。

为了证明上面代码的正确性，我们写一个简单的单元测试，替换掉自带的 `it_works` 函数：

```rust,noplaypen
{{#include ../../zcore/src/object/mod.rs:dummy_test}}
```

```sh
$ cargo test
    Finished test [unoptimized + debuginfo] target(s) in 0.53s
     Running target/debug/deps/zcore-ae1be84852989b13

running 1 test
test tests::dummy_object ... ok
```

大功告成！让我们用 `cargo fmt` 命令格式化一下代码，然后记得 `git commit` 及时保存进展。

## 实现接口到具体类型的向下转换

在系统调用中，用户进程会传入一个内核对象的句柄，然后内核会根据系统调用的类型，尝试将其转换成特定类型的对象。
于是这里产生了一个很重要的需求：将接口 `Arc<dyn KernelObject>` 转换成具体类型的结构 `Arc<T> where T: KernelObject`。
这种操作在面向对象语言中称为**向下转换（downcast）**。

在大部分编程语言中，向下转换都是一件非常轻松的事情。例如在 C/C++ 中，我们可以这样写：

```c++
struct KernelObject {...};
struct DummyObject: KernelObject {...};

KernelObject *base = ...;
// C 风格：强制类型转换
DummyObject *dummy = (DummyObject*)(base);
// C++ 风格：动态类型转换
DummyObject *dummy = dynamic_cast<DummyObject*>(base);
```

但在 Rust 中，由于其 trait 模型的限制，向下转换并不是一件容易的事情。
虽然标准库中提供了 [`Any`] trait，部分实现了动态类型的功能，但实际操作起来却困难重重。
不信邪的同学可以自己折腾一下：

[`Any`]: https://doc.rust-lang.org/std/any/

```rust,editable
#use std::any::Any;
#use std::sync::Arc;
trait KernelObject: Any + Send + Sync {}
fn downcast_v1<T: KernelObject>(object: Arc<dyn KernelObject>) -> Arc<T> {
    object.downcast::<T>().unwrap()
}
fn downcast_v2<T: KernelObject>(object: Arc<dyn KernelObject>) -> Arc<T> {
    let object: Arc<dyn Any + Send + Sync + 'static> = object;
    object.downcast::<T>().unwrap()
}
```

当然这个问题也困扰了 Rust 社区中的很多人。目前已经有人提出了一套不错的解决方案，就是我们接下来要引入的 [`downcast-rs`] 库：

[`downcast-rs`]: https://docs.rs/downcast-rs/1.2.0/downcast_rs/index.html

```toml
[dependencies]
{{#include ../../zcore/Cargo.toml:downcast}}
```

（题外话：这个库原来是不支持 no_std 的，zCore 有这个需求，于是就顺便帮他实现了一把）

按照它文档的描述，我们要为自己的接口实现向下转换，只需以下修改：

```rust,noplaypen
use core::fmt::Debug;
use downcast_rs::{impl_downcast, DowncastSync};

pub trait KernelObject: DowncastSync + Debug {...}
impl_downcast!(sync KernelObject);
```

其中 `DowncastSync` 代替了原来的 `Send + Sync`，`Debug` 用于出错时输出调试信息。
`impl_downcast!` 宏用来帮我们自动生成转换函数，然后就可以用 `downcast_arc` 来对 `Arc` 做向下转换了。我们直接来测试一把：

```rust,noplaypen
{{#include ../../zcore/src/object/mod.rs:downcast_test}}
```

```sh
$ cargo test
    Finished test [unoptimized + debuginfo] target(s) in 0.47s
     Running target/debug/deps/zcore-ae1be84852989b13

running 2 tests
test object::downcast ... ok
test object::tests::dummy_object ... ok
```

## 用宏自动生成 `impl KernelObject` 模板代码

传统 OOP 语言都支持 **继承（inheritance）** 功能：子类 B 可以继承父类 A，然后自动拥有父类的所有字段和方法。

> 继承野蛮，trait 文明。 —— 某 Rust 爱好者

```rust,noplaypen
{{#include ../../zcore/src/object/mod.rs:base}}
```

## 总结

关于 Rust 的面向对象特性，可以参考[官方文档]。

[官方文档]: https://kaisery.github.io/trpl-zh-cn/ch17-00-oop.html
