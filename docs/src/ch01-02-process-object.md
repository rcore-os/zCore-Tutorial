# 对象管理器：Process 对象

## 句柄和权限

[句柄]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/handles.md
[权限]: https://github.com/zhangpf/fuchsia-docs-zh-CN/blob/master/zircon/docs/rights.md

> 介绍并实现 Handle，Rights

## 实现第一个内核对象

> 使用上一节的方法，实现一个空的 Process 对象

## 存储内核对象句柄

> 添加成员变量 handles: BTreeMap<HandleValue, Handle>
>
> 实现 create，add_handle，remove_handle 函数

## 根据句柄查找内核对象

> 实现 get_object_with_rights 等其它相关函数
>
> 实现 handle 单元测试
