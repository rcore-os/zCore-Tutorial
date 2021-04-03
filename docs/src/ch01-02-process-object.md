# 对象管理器：Process 对象

## 句柄和权限

[句柄]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/handles.md
[权限]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/rights.md

> 介绍并实现 Handle，Rights

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
