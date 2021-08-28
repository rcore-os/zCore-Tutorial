# Zircon 系统调用
> 目录位于`zCore/zircon-syscall`

从userboot运行起来到实现调用syscall的简要函数调用流程如下：

1.  run_userboot    ->       
2.  proc.start      ->
3.  thread_fn       ->
4.  new_thread      -> 
5.  handle_syscall  -> 
6.  syscall         ->
7.  sys_handle_close()   (举例某一具体的syscall运行，该syscall可用于实现`close a handle`的功能)

## 获取系统调用参数
从寄存器中获取参数  
> 不同的计算机体系结构获得参数的方式不同  
> 
> 以下区分`x86_64`以及`aarch64`  

调用syscall需要从寄存器收集两种参数：  
+ `num` : 系统调用号  
+ `args` : 具体某一系统调用的参数

```rust
async fn handle_syscall(thread: &CurrentThread, regs: &mut GeneralRegs) {
    #[cfg(target_arch = "x86_64")]
    let num = regs.rax as u32;
    #[cfg(target_arch = "aarch64")]
    let num = regs.x16 as u32;
    // LibOS: Function call ABI
    #[cfg(feature = "std")]
    #[cfg(target_arch = "x86_64")]
    let args = unsafe {
        let a6 = (regs.rsp as *const usize).read();
        let a7 = (regs.rsp as *const usize).add(1).read();
        [
            regs.rdi, regs.rsi, regs.rdx, regs.rcx, regs.r8, regs.r9, a6, a7,
        ]
    };
    // RealOS: Zircon syscall ABI
    #[cfg(not(feature = "std"))]
    #[cfg(target_arch = "x86_64")]
    let args = [
        regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9, regs.r12, regs.r13,
    ];
    // ARM64
    #[cfg(target_arch = "aarch64")]
    let args = [
        regs.x0, regs.x1, regs.x2, regs.x3, regs.x4, regs.x5, regs.x6, regs.x7,
    ];
    let mut syscall = Syscall {
        regs,
        thread,
        thread_fn,
    };
    let ret = syscall.syscall(num, args).await as usize;
    #[cfg(target_arch = "x86_64")]
    {
        syscall.regs.rax = ret;
    }
    #[cfg(target_arch = "aarch64")]
    {
        syscall.regs.x0 = ret;
    }
}

```



## 系统调用上下文与处理函数  

### 定义 Syscall 结构体  

保存上下文信息  

> zCore/zircon-syscall/src/lib.rs#L52

```rust
/// 系统调用的结构(存储关于创建系统调用的信息)
pub struct Syscall<'a> {
    /// store the regs statues
    pub regs: &'a mut GeneralRegs,
    /// the thread making a syscall
    pub thread: &'a CurrentThread,
    /// new thread function
    pub thread_fn: ThreadFn,
}
```  

### 实现 syscall 函数

> zCore/zircon-syscall/src/lib.rs#L59   

1. 检查系统调用号`sys_type`是否合法  
2. 获取传递给具体某一系统调用的参数`args`  
3. 若syscall函数输入的系统调用号合法，则进一步根据系统调用号匹配具体系统调用处理函数
4. 传入对应系统调用所需的参数，并运行之  
5. 检查系统调用的返回值`ret`是否符合预期 

```rust
    pub async fn syscall(&mut self, num: u32, args: [usize; 8]) -> isize {

        ...

        // 1. 检查系统调用号`sys_type`是否合法
        let sys_type = match Sys::try_from(num) {
            Ok(t) => t,
            Err(_) => {
                error!("invalid syscall number: {}", num);
                return ZxError::INVALID_ARGS as _;
            }
        };

        ...

        // 2. 获取传递给具体系统调用参数
        let [a0, a1, a2, a3, a4, a5, a6, a7] = args;

        // 3. 若syscall函数输入的系统调用号合法
        //    则进一步根据系统调用号匹配具体系统调用处理函数

        let ret = match sys_type {
            
            // 4. 传入对应系统调用所需的参数，并运行之
            Sys::HANDLE_CLOSE => self.sys_handle_close(a0 as _),
            Sys::HANDLE_CLOSE_MANY => self.sys_handle_close_many(a0.into(), a1 as _),
            Sys::HANDLE_DUPLICATE => self.sys_handle_duplicate(a0 as _, a1 as _, a2.into()),
            Sys::HANDLE_REPLACE => self.sys_handle_replace(a0 as _, a1 as _, a2.into()),
            
            ...
            // 更多系统调用匹配的分支
            Sys::CLOCK_GET => self.sys_clock_get(a0 as _, a1.into()),
            Sys::CLOCK_READ => self.sys_clock_read(a0 as _, a1.into()),
            Sys::CLOCK_ADJUST => self.sys_clock_adjust(a0 as _, a1 as _, a2 as _),
            Sys::CLOCK_UPDATE => self.sys_clock_update(a0 as _, a1 as _, a2.into()),
            Sys::TIMER_CREATE => self.sys_timer_create(a0 as _, a1 as _, a2.into()),

            ...
        };

        ...

        // 5. 检查系统调用的返回值`ret`是否符合预期
        match ret {
            Ok(_) => 0,
            Err(err) => err as isize,
        }
    }
```


系统调用号匹配信息位于
> zCore/zircon-syscall/src/consts.rs#L8
```rust
pub enum SyscallType {
    BTI_CREATE = 0,
    BTI_PIN = 1,
    BTI_RELEASE_QUARANTINE = 2,
    CHANNEL_CREATE = 3,
    CHANNEL_READ = 4,
    CHANNEL_READ_ETC = 5,
    CHANNEL_WRITE = 6,
    CHANNEL_WRITE_ETC = 7,
    CHANNEL_CALL_NORETRY = 8,
    CHANNEL_CALL_FINISH = 9,
    CLOCK_GET = 10,
    CLOCK_ADJUST = 11,
    CLOCK_GET_MONOTONIC_VIA_KERNEL = 12,

    ...

    VMO_CREATE_CONTIGUOUS = 165,
    VMO_CREATE_PHYSICAL = 166,
    COUNT = 167,
    FUTEX_WAKE_HANDLE_CLOSE_THREAD_EXIT = 200,
    VMAR_UNMAP_HANDLE_CLOSE_THREAD_EXIT = 201,
}
```



### 简单实现一个系统调用处理函数（`sys_clock_adjust`为例）

```rust
    pub fn sys_clock_adjust(&self, resource: HandleValue, clock_id: u32, offset: u64) -> ZxResult {

    // 1. 记录log信息：info!()
        info!(
            "clock.adjust: resource={:#x?}, id={:#x}, offset={:#x}",
            resource, clock_id, offset
        );

    // 2. 检查参数合法性（需要归纳出每个系统调用的参数值的范围）
    // missing now

    // 3. 获取当前进程对象
        let proc = self.thread.proc();

    // 4. 根据句柄从进程中获取对象
        proc.get_object::<Resource>(resource)?
            .validate(ResourceKind::ROOT)?;
        match clock_id {
            ZX_CLOCK_MONOTONIC => Err(ZxError::ACCESS_DENIED),

    // 5. 调用内河对象API执行具体功能
            ZX_CLOCK_UTC => {
                UTC_OFFSET.store(offset, Ordering::Relaxed);
                Ok(())
            }
            _ => Err(ZxError::INVALID_ARGS),
        }
    }
```


> 一个现有不完全的系统调用实现`sys_clock_get`

```rust
    /// Acquire the current time.  
    ///   
    /// + Returns the current time of clock_id via `time`.  
    /// + Returns whether `clock_id` was valid.  
    pub fn sys_clock_get(&self, clock_id: u32, mut time: UserOutPtr<u64>) -> ZxResult {
        // 记录log信息：info!()
        info!("clock.get: id={}", clock_id); 
        // 检查参数合法性
        // miss

        match clock_id {
            ZX_CLOCK_MONOTONIC => {
                time.write(timer_now().as_nanos() as u64)?;
                Ok(())
            }
            ZX_CLOCK_UTC => {
                time.write(timer_now().as_nanos() as u64 + UTC_OFFSET.load(Ordering::Relaxed))?;
                Ok(())
            }
            ZX_CLOCK_THREAD => {
                time.write(self.thread.get_time())?;
                Ok(())
            }
            _ => Err(ZxError::NOT_SUPPORTED),
        }
    }
```
