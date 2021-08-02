#### 对象管理器：Process 对象

## 权限

[权限]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/rights.md

内核对象的“[权限](https://fuchsia.dev/docs/concepts/kernel/rights)”指定允许对内核对象进行哪些操作。权限与句柄相关联，并传达对关联句柄或与句柄关联的对象执行操作的特权。单个进程可能对具有不同权限的同一个内核对象有两个不同的句柄。

## 句柄

[句柄]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/handles.md


句柄是允许用户程序引用内核对象引用的一种内核结构，它可以被认为是与特定内核对象的会话或连接。

通常情况下，多个进程通过不同的句柄同时访问同一个对象。对象可能有多个句柄（在一个或多个进程中）引用它们。但单个句柄只能绑定到单个进程或绑定到内核。

当句柄绑定到内核时，我们说它是“在传输中”（'in-transit'）。

在用户模式下，句柄只是某个系统调用返回的特定数字。只有“不在传输中”的句柄对用户模式可见。

代表句柄的整数只对其所属的那个进程有意义。另一个进程中的相同数字可能不会映射到任何句柄，或者它可能映射到指向完全不同的内核对象的句柄。

句柄的整数值是任何 32 位数字，但对应于**ZX_HANDLE_INVALID**的值将始终为 0。除此之外，有效句柄的整数值将始终具有句柄集的两个最低有效位. 可以使用**ZX_HANDLE_FIXED_BITS_MASK**访问代表这些位的掩码。

句柄可以从一个进程移动到另一个进程，方法是将它们写入通道（使用[`channel_write()`](https://fuchsia.dev/docs/reference/syscalls/channel_write)），或者使用 [`process_start()`](https://fuchsia.dev/docs/reference/syscalls/process_start)传递一个句柄作为新进程中第一个线程的参数。对于几乎所有的对象，当最后一个打开的引用对象的句柄关闭时，对象要么被销毁，要么被置于可能无法撤消的最终状态。



在 `Cargo.toml` 中加入 `bitflags` 库：

```rust,noplaypen
[dependencies]
{{#include ../../code/ch01-02/Cargo.toml:bitflags}}
```

在 object 模块下定义两个子模块：

```rust,noplaypen
// src/object/mod.rs
{{#include ../../code/ch01-02/src/object/mod.rs:mod}}
```

定义权限：

```rust,noplaypen
// src/object/rights.rs
{{#include ../../code/ch01-02/src/object/rights.rs:rights}}
```

定义句柄：

```rust,noplaypen
// src/object/handle.rs
{{#include ../../code/ch01-02/src/object/handle.rs:handle}}
```

## 存储内核对象句柄

> 添加成员变量 handles: BTreeMap<HandleValue, Handle>
>
> 实现 create，add_handle，remove_handle 函数

使用上一节的方法，实现一个空的 Process 对象：

```rust,noplaypen
// src/task/process.rs
{{#include ../../code/ch01-02/src/task/process.rs:process}}
}
```

插入、删除句柄函数：

```rust,noplaypen
// src/task/process.rs
impl Process {
{{#include ../../code/ch01-02/src/task/process.rs:add_remove_handle}}
}
```

## 定义内核错误及 `Result` 类型

```rust,noplaypen
// src/error.rs
{{#include ../../code/ch01-02/src/error.rs:error_begin}}

    // ......

{{#include ../../code/ch01-02/src/error.rs:error_end}}
```

```rust,noplaypen
// src/error.rs
{{#include ../../code/ch01-02/src/error.rs:result}}
```

## 根据句柄查找内核对象

> 实现 get_object_with_rights 等其它相关函数
>
> 实现 handle 单元测试

```rust,noplaypen
// src/task/process.rs
impl Process {
{{#include ../../code/ch01-02/src/task/process.rs:get_object_with_rights}}
}
```
