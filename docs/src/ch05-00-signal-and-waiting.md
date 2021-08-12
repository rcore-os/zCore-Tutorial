# 信号和等待

## 信号

对象可能有多达 32 个信号（由 zx_signals *t 类型和 ZX* **SIGNAL** 定义表示），它们表示有关其当前状态的一条信息。例如，通道和套接字可能是 READABLE 或 WRITABLE 的。进程或线程可能会被终止。等等。

线程可以等待信号在一个或多个对象上变为活动状态。

## 等待

线程可用于[`zx_object_wait_one()`](https://fuchsia.dev/docs/reference/syscalls/object_wait_one) 等待单个句柄上的信号处于活动状态或 [`zx_object_wait_many()`](https://fuchsia.dev/docs/reference/syscalls/object_wait_many)等待多个句柄上的信号。两个调用都允许超时，即使没有信号挂起，它们也会返回。

超时可能会偏离指定的截止时间，具体取决于计时器的余量。

如果线程要等待大量句柄，使用端口（Port）会更有效，它是一个对象，其他对象可能会绑定到这样的对象，当信号在它们上被断言时，端口会收到一个包含信息的数据包关于未决信号。

## 事件与事件对

事件（Event）是最简单的对象，除了它的活动信号集合之外没有其他状态。

事件对（Event Pair）是可以相互发出信号的一对事件中的一个。事件对的一个有用属性是，当一对的一侧消失时（它的所有句柄都已关闭），PEER_CLOSED 信号在另一侧被断言。

见：[`zx_event_create()`](https://fuchsia.dev/docs/reference/syscalls/event_create), 和[`zx_eventpair_create()`](https://fuchsia.dev/docs/reference/syscalls/eventpair_create)。