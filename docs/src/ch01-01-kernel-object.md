# 初识内核对象

## 内核对象简介

在动手编写我们的代码之前，需要首先进行调研和学习，对目标对象有一个全面系统的了解。
而了解一个项目设计的最好方式就是阅读官方提供的手册和文档。

让我们先来阅读一下 Fuchsia 官方文档：[内核对象]。这个链接是社区翻译的中文版，已经有些年头了。如果读者能够科学上网，推荐直接阅读[官方英文版]。

[内核对象]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/objects.md
[官方英文版]: https://fuchsia.dev/fuchsia-src/reference/kernel_objects/objects

通过阅读文档，我们了解到与内核对象相关的三个重要概念：**对象（Object），句柄（Handle），权限（Rights）**。它们在 Zircon 内核中的角色和关系如下图所示：

![](img/ch01-01-kernel-object.png)

简单来说：

* Zircon是一个基于对象的内核，内核资源被抽象封装在不同的 **对象** 中。
* 用户程序通过 **句柄** 与内核交互。句柄是对某一对象的引用，并且附加了特定的 **权限**。
* 对象通过 **引用计数** 管理生命周期。当最后一个句柄关闭时，对象随之销毁。

此外在内核对象的文档中，还列举了一些[常用对象]。点击链接进去就能查看到这个对象的[具体描述]，在页面最下方还列举了与这个对象相关的[全部系统调用]。
进一步查看系统调用的 [API 定义]，以及它的[行为描述]，我们就能更深入地了解用户程序操作内核对象的一些细节：

[常用对象]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/objects.md#应用程序可用的内核对象
[具体描述]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/objects/channel.md
[全部系统调用]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/objects/channel.md#系统调用
[API 定义]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/syscalls/channel_read.md#概要
[行为描述]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/syscalls/channel_read.md#描述

* 创建：每一种内核对象都存在一个系统调用来创建它，例如 [`zx_channel_create`]。
创建对象时一般需要传入一个参数选项 `options`，若创建成功则内核会将一个新句柄写入用户指定的内存中。

* 使用：获得对象句柄后可以通过若干系统调用对它进行操作，例如 [`zx_channel_write`]。
这类系统调用一般需要传入句柄 `handle` 作为第一个参数，内核首先对其进行检查，如果句柄非法或者对象类型与系统调用不匹配就会报错。
接下来内核会检查句柄的权限是否满足操作的要求，例如 `write` 操作一般要求句柄具有 `WRITE` 权限，如果权限不满足就会继续报错。

* 关闭：当用户程序不再使用对象时，会调用 [`zx_handle_close`] 关闭句柄。当用户进程退出时，仍处于打开状态的句柄也都会自动关闭。

[`zx_channel_create`]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/syscalls/channel_create.md
[`zx_channel_write`]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/syscalls/channel_write.md
[`zx_handle_close`]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/syscalls/handle_close.md

我们还发现，有一类 Object 系统调用是对所有内核对象都适用的。
这表明所有内核对象都有一些公共属性，例如 ID、名称等等。每一种内核对象也会有自己特有的属性。

其中一些 Object 系统调用和信号相关。Zircon 每个内核对象都附带有 32 个 **[信号（Signals）]**，它们代表了不同类型的事件。
与传统 Unix 系统的信号不同，它不能异步地打断用户程序运行，而只能由用户程序主动地阻塞等待在某个对象的某些信号上面。
信号是 Zircon 内核中很重要的机制，不过这部分在前期不会涉及，我们留到第五章再具体实现。

[信号（Signals）]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/signals.md

以上我们了解了 Zircon 内核对象的相关概念和使用方式。接下来在这一节中，我们将用 Rust 实现内核对象的基本框架，以方便后续快速实现各种具体类型的内核对象。
从传统面向对象语言的视角看，我们只是在实现一个基类。但由于 Rust 语言模型的限制，这件事情需要用到一些特殊的技巧。

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
// src/lib.rs
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

```rust,noplaypen
use alloc::string::String;
// src/object/mod.rs
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
// src/object/object.rs
{{#include ../../zcore/src/object/object_v1.rs:dummy_def}}
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
// src/object/object.rs
{{#include ../../zcore/src/object/object_v1.rs:dummy_new}}
```

根据文档描述，每个内核对象都有唯一的 ID。为此我们需要实现一个全局的 ID 分配方法。这里采用的方法是用一个静态变量存放下一个待分配 ID 值，每次分配就原子地 +1。
ID 类型使用 `u64`，保证了数值空间足够大，在有生之年都不用担心溢出问题。在 Zircon 中 ID 从 1024 开始分配，1024 以下保留作内核内部使用。

另外注意这里 `new` 函数返回类型不是 `Self` 而是 `Arc<Self>`，这是为了以后方便而做的统一约定。

最后我们为它实现 `KernelObject` 接口：

```rust,noplaypen
// src/object/object.rs
{{#include ../../zcore/src/object/object_v1.rs:dummy_impl}}
```

到此为止，我们已经迈出了万里长征第一步，实现了一个最简单的功能。有实现，就要有测试！即使最简单的代码也要保证它的行为符合我们预期。
只有对现有代码进行充分测试，在未来做添加和修改的时候，我们才有信心不会把事情搞砸。俗话讲"万丈高楼平地起"，把地基打好才能盖摩天大楼。

为了证明上面代码的正确性，我们写一个简单的单元测试，替换掉自带的 `it_works` 函数：

```rust,noplaypen
// src/object/object.rs
{{#include ../../zcore/src/object/object_v1.rs:dummy_test}}
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
# use std::any::Any;
# use std::sync::Arc;
# fn main() {}

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

（题外话：这个库原来是不支持 `no_std` 的，zCore 有这个需求，于是就顺便帮他实现了一把）

按照它文档的描述，我们要为自己的接口实现向下转换，只需以下修改：

```rust,noplaypen
// src/object/mod.rs
use core::fmt::Debug;
use downcast_rs::{impl_downcast, DowncastSync};

pub trait KernelObject: DowncastSync + Debug {...}
impl_downcast!(sync KernelObject);
```

其中 `DowncastSync` 代替了原来的 `Send + Sync`，`Debug` 用于出错时输出调试信息。
`impl_downcast!` 宏用来帮我们自动生成转换函数，然后就可以用 `downcast_arc` 来对 `Arc` 做向下转换了。我们直接来测试一把：

```rust,noplaypen
// src/object/object.rs
{{#include ../../zcore/src/object/object_v1.rs:downcast_test}}
```

```sh
$ cargo test
    Finished test [unoptimized + debuginfo] target(s) in 0.47s
     Running target/debug/deps/zcore-ae1be84852989b13

running 2 tests
test object::downcast ... ok
test object::tests::dummy_object ... ok
```

## 模拟继承：用宏自动生成接口实现代码

上面我们已经完整实现了一个内核对象，代码看起来很简洁。但当我们要实现更多对象的时候，就会发现一个问题：
这些对象拥有一些公共属性，接口方法也有共同的实现。
在传统 OOP 语言中，我们通常使用 **继承（inheritance）** 来复用这些公共代码：子类 B 可以继承父类 A，然后自动拥有父类的所有字段和方法。

继承是一个很强大的功能，但在长期实践中人们也逐渐发现了它的弊端。有兴趣的读者可以看一看知乎上的探讨：[*面向对象编程的弊端是什么？*]。
经典著作《设计模式》中就鼓励大家**使用组合代替继承**。而一些现代的编程语言，如 Go 和 Rust，甚至直接抛弃了继承。在 Rust 中，通常使用组合结构和 [`Deref`] trait 来部分模拟继承。

[*面向对象编程的弊端是什么？*]: https://www.zhihu.com/question/20275578/answer/26577791
[`Deref`]: https://kaisery.github.io/trpl-zh-cn/ch15-02-deref.html

> 继承野蛮，trait 文明。 —— 某 Rust 爱好者

接下来我们模仿 `downcast-rs` 库的做法，使用一种基于宏的代码生成方案，来实现 `KernelObject` 的继承。
当然这只是抛砖引玉，如果读者自己实现了，或者了解到社区中有更好的解决方案，也欢迎指出。

具体做法是这样的：

- 使用一个 struct 来提供所有的公共属性和方法，作为所有子类的第一个成员。
- 为子类实现 trait 接口，所有方法直接委托给内部 struct 完成。这部分使用宏来自动生成模板代码。

而所谓的内部 struct，其实就是我们上面实现的 `DummyObject`。为了更好地体现它的功能，我们给他改个名叫 `KObjectBase`：

```rust,noplaypen
// src/object/mod.rs
{{#include ../../zcore/src/object/mod.rs:base_def}}
```

接下来我们把它的构造函数改为实现 `Default` trait，并且公共属性和方法都指定为 `pub`：

```rust,noplaypen
// src/object/mod.rs
{{#include ../../zcore/src/object/mod.rs:base_default}}
impl KObjectBase {
    /// 生成一个唯一的 ID
    fn new_koid() -> KoID {...}
    /// 获取对象名称
    pub fn name(&self) -> String {...}
    /// 设置对象名称
    pub fn set_name(&self, name: &str) {...}
}
```

最后来写一个魔法的宏！

```rust,noplaypen
// src/object/mod.rs
{{#include ../../zcore/src/object/mod.rs:impl_kobject}}
```

轮子已经造好了！让我们看看如何用它方便地实现一个内核对象，仍以 `DummyObject` 为例：

```rust,noplaypen
// src/object/mod.rs
{{#include ../../zcore/src/object/mod.rs:dummy}}
```

是不是方便了很多？最后按照惯例，用单元测试检验实现的正确性：

```rust,noplaypen
// src/object/mod.rs
{{#include ../../zcore/src/object/mod.rs:dummy_test}}
```

有兴趣的读者可以继续探索使用功能更强大的 [**过程宏（proc_macro）**]，进一步简化实现新内核对象所需的模板代码。
如果能把上面的代码块缩小成下面这两行，就更加完美了：

[**过程宏（proc_macro）**]: https://doc.rust-lang.org/proc_macro/index.html

```rust,noplaypen
#[KernelObject]
pub struct DummyObject;
```

## 总结

在这一节中我们用 Rust 语言实现了 Zircon 最核心的**内核对象**概念。在此过程中涉及到 Rust 的一系列语言特性和设计模式：

- 使用 **trait** 实现接口
- 使用 **内部可变性** 模式实现并发对象
- 基于社区解决方案实现 trait 到 struct 的 **向下转换**
- 使用组合模拟继承，并使用 **宏** 实现模板代码的自动生成

由于 Rust 独特的[面向对象编程特性]，我们在实现内核对象的过程中遇到了一定的挑战。
不过万事开头难，解决这些问题为整个项目打下了坚实基础，后面实现新的内核对象就会变得简单很多。

[面向对象编程特性]: https://kaisery.github.io/trpl-zh-cn/ch17-00-oop.html

在下一节中，我们将介绍内核对象相关的另外两个概念：句柄和权限，并实现内核对象的存储和访问。
