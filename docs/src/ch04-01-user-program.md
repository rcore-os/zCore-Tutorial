# Zircon 用户程序

## 用户态启动流程

### 流程概要
 kernel   
 -> userboot  (decompress bootsvc LZ4 format)   
 -> bootsvc   (可执行文件bin/component_manager)  
 -> component_manager   
 -> sh / device_manager  

### ZBI(Zircon Boot Image)
ZBI是一种简单的容器格式，它内嵌了许多可由引导加载程序 `BootLoader`传递的项目内容，包括硬件特定的信息、提供引导选项的内核“命令行”以及RAM磁盘映像(通常是被压缩的)。`ZBI`中包含了初始文件系统 `bootfs`，内核将 `ZBI` 完整传递给 `userboot`，由它负责解析并对其它进程提供文件服务。


### bootfs

基本的`bootfs`映像可满足用户空间程序运行需要的所有依赖:
+ 可执行文件
+ 共享库
+ 数据文件  
  
以上列出的内容还可实现设备驱动或更高级的文件系统，从而能够从存储设备或网络设备上访问读取更多的代码和数据。

在系统自引导结束后，`bootfs`中的文件就会成为一个挂载在根目录`/boot`上的只读文件系统树(并由bootsvc提供服务)。随后`userboot`将从`bootfs`加载第一个真正意义上的用户程序。

### [zCore程序(ELF加载与动态链接)](https://fuchsia.dev/fuchsia-src/concepts/booting/program_loading)

zCore内核不直接参与正常程序的加载，而是提供了一些用户态程序加载时可用的模块。如虚拟内存对象(VMO)、进程(processes)、虚拟地址空间（VMAR）和线程(threads)。 


### ELF 格式以及系统应用程序二进制接口(system ABI)


标准的zCore用户空间环境提供了动态链接器以及基于ELF的执行环境，能够运行ELF格式的格式机器码可执行文件。zCore进程只能通过zCore vDSO使用系统调用。内核采用基于ELF系统常见的程序二进制接口(ABI)提供了vDSO。  

具备适当功能的用户空间代码可通过系统调用直接创建进程和加载程序，而不用ELF。但是zCore的标准ABI使用了这里所述的ELF。有关ELF文件格式的背景知识如下： 

### ELF文件类型 

“ET_REL”代表此ELF文件为可重定位文件  

“ET_EXEC“代表ELF文件为可执行文件  
 
“ET_DYN”代表此ELF文件为动态链接库  

“ET_CORE”代表此ELF文件是核心转储文件  


### 传统ELF程序文件加载  

可执行链接格式(Executable and Linking Format, ELF)最初由 UNIX 系统实验室开发并发布，并成为大多数类Unix系统的通用标准可执行文件格式。在这些系统中，内核使用```POSIX```(可移植操作系统接口)```execve API```将程序加载与文件系统访问集成在一起。该类系统加载ELF程序的方式会有一些不同，但大多遵循以下模式:  


1. 内核按照名称加载文件，并检查它是ELF还是系统支持的其他类型的文件。  


2. 内核根据ELF文件的```PT_LOAD```程序头来映射ELF映像。对于```ET_EXEC```文件，系统会将程序中的各段(Section)放到```p_vaddr```中所指定内存中的固定地址。对于```ET_DYN```文件，系统将加载程序第一个```PT_LOAD```的基地址，然后根据它们的```p_vaddr```相对于第一个section的```p_vaddr```放置后面的section。 通常来说该基地址是通过地址随机化(ASLR)来产生的。  


3. 若ELF文件中有一个```PT_INTERP```(Program interpreter)程序头,  它的部分内容(ELF文件中```p_offset```和```p_filesz```给出的一些字节)被当做为一个文件名，改文件名用于寻找另一个称为“ELF解释器”的ELF文件。上述这种ELF文件是```ET_DYN```文件。内核采用同样的方式将该类ELF文件加载，但是所加载的地址是自定的。该ELF“解释器”通常指的是被命名为```/lib/ld.so.1``` 或者是 ```/lib/ld-linux.so.2```的ELF动态链接器。



4. 内核为初始的线程设置了寄存器和堆栈的内容，并在PC寄存器已指向特定程序入口处(Entry Point)的情况下启动线程。 
    + 程序入口处(Entry Point)指的是ELF文件头中 ```e_entry```的值，它会根据程序基地址(base address)做相应的调整。如果这是一个带有```PT_INTERP```的ELF文件，则它的入口点不在它本身，而是被设置在动态链接器中。
    + 内核通过设置寄存器和堆栈来使得程序能够接收特定的参数，环境变量以及其它有实际用途的辅助向量。寄存器和堆栈的设置方法遵循了一种汇编级别的协议方式。若ELF文件运行时依赖动态链接，即带有```PT_INTERP```。则寄存器和堆栈中将包括来自该可执行文件的ELF文件头中的基地址、入口点和程序头部表地址信息，这些信息可允许动态链接器在内存中找到该可执行文件的ELF动态链接元数据，以实现动态链接。当动态链接启动完成后，动态链接器将跳转到该可执行文件的入口点地址。

    ```rust
        pub fn sys_process_start(
            &self,
            proc_handle: HandleValue,
            thread_handle: HandleValue,
            entry: usize,
            stack: usize,
            arg1_handle: HandleValue,
            arg2: usize,
        ) -> ZxResult {
            info!("process.start: proc_handle={:?}, thread_handle={:?}, entry={:?}, stack={:?}, arg1_handle={:?}, arg2={:?}",
                proc_handle, thread_handle, entry, stack, arg1_handle, arg2
            );
            let proc = self.thread.proc();
            let process = proc.get_object_with_rights::<Process>(proc_handle, Rights::WRITE)?;
            let thread = proc.get_object_with_rights::<Thread>(thread_handle, Rights::WRITE)?;
            if !Arc::ptr_eq(&thread.proc(), &process) {
                return Err(ZxError::ACCESS_DENIED);
            }
            let arg1 = if arg1_handle != INVALID_HANDLE {
                let arg1 = proc.remove_handle(arg1_handle)?;
                if !arg1.rights.contains(Rights::TRANSFER) {
                    return Err(ZxError::ACCESS_DENIED);
                }
                Some(arg1)
            } else {
                None
            };
            process.start(&thread, entry, stack, arg1, arg2, self.spawn_fn)?;
            Ok(())
        }
    ```
zCore的程序加载受到了传统方式的启发，但是有一些不同。在传统模式中，需要在加载动态链接器之前加载可执行文件的一个关键原因是，动态链接器随机化选择的基地址(base address)不能与```ET_EXEC```可执行文件使用的固定地址相交。zCore从根本上并不支持```ET_EXEC```格式ELF文件的固定地址程序加载，它只支持位置无关的可执行文件或[PIE](https://patchwork.kernel.org/patch/9807325/)(```ET_DYN```格式的ELF文件)


### VmarExt trait实现 

zCore底层的API不支持文件系统。zCore程序文件的加载通过虚拟内存对象(VMO)以及```channel```使用的进程间通信机制来完成。

程序的加载基于如下一些前提：
+ 获得一个包含可执行文件的虚拟内存对象（VMO）的句柄。

> zircon-object\src\util\elf_loader.rs
```shell
fn make_vmo(elf: &ElfFile, ph: ProgramHeader) -> ZxResult<Arc<VmObject>> {
    assert_eq!(ph.get_type().unwrap(), Type::Load);
    let page_offset = ph.virtual_addr() as usize % PAGE_SIZE;
    let pages = pages(ph.mem_size() as usize + page_offset);
    let vmo = VmObject::new_paged(pages);
    let data = match ph.get_data(&elf).unwrap() {
        SegmentData::Undefined(data) => data,
        _ => return Err(ZxError::INVALID_ARGS),
    };
    vmo.write(page_offset, data)?;
    Ok(vmo)
}
```
+ 程序执行参数列表。
+ 程序执行环境变量列表。
+ 存在一个初始的句柄列表，每个句柄都有一个句柄信息项。


### USERBOOT

#### 使用userboot的原因 

在Zircon中，内嵌在ZBI中的`RAM磁盘映像`通常采用[LZ4](https://github.com/lz4/lz4)格式压缩。解压后将继续得到`bootfs`格式的磁盘镜像。这是一种简单的只读文件系统格式，它只列出文件名。且对于每个文件，可分别列出它们在BOOTFS映像中的偏移量和大小(这两个值都必须是页面对齐的，并且限制在32位)。

由于kernel中没有包含任何可用于解压缩[LZ4](https://github.com/lz4/lz4)格式的代码，也没有任何用于解析BOOTFS格式的代码。所有这些工作都是由称为`userboot`的第一个用户空间进程完成的。


> zCore中未找到解压缩bootfs的相关实现，  
> 但是能够在scripts/gen-prebuilt.sh中找到ZBI中确实有bootfs的内容  
> 且现有的zCore实现中有关所载入的ZBI方式如下：  

> zircon-loader/src/lib.rs
```rust
    // zbi
    let zbi_vmo = {
        let vmo = VmObject::new_paged(images.zbi.as_ref().len() / PAGE_SIZE + 1);
        vmo.write(0, images.zbi.as_ref()).unwrap();
        vmo.set_name("zbi");
        vmo
    };
```
#### userboot是什么
userboot是一个普通的用户空间进程。它只能像任何其他进程一样通过vDSO执行标准的系统调用，并受完整vDSO执行制度的约束。

> 唯一一个由内核态“不规范地”创建的用户进程   
> 
> userboot具体实现的功能有：  
> 
> + 读取channel中的cmdline、handles 
> 
> + 解析zbi
> 
> + 解压BOOTFS 
> 
> + 选定下一个程序启动 自己充当loader，然后“死亡”  
> 
> + 用“规范的方式”启动下一个程序


userboot被构建为一个ELF动态共享对象(DSO,dynamic shared object)，使用了与vDSO相同的布局。与vDSO一样，userboot的ELF映像在编译时就被嵌入到内核中。其简单的布局意味着加载它不需要内核在引导时解析ELF的文件头。内核只需要知道三件事:
1. 只读段`segment`的大小
2. 可执行段`segment`的大小
3. `userboot`入口点的地址。  
   
这些值在编译时便可从userboot ELF映像中提取，并在内核代码中用作常量。

#### kernel如何启用userboot

与任何其他进程一样，userboot必须从已经映射到其地址空间的vDSO开始，这样它才能进行系统调用。内核将userboot和vDSO映射到第一个用户进程，然后在userboot的入口处启动它。

<!-- > !  userboot的特殊之处在于它的加载方式。   
> ...todo -->

#### userboot如何在vDSO中取得系统调用
当内核将`userboot`映射到第一个用户进程时，会像正常程序那样，在内存中选择一个随机地址进行加载。而在映射`userboot`的vDSO时，并不采用上述随机的方式，而是将vDSO映像直接放在内存中`userboot`的映像之后。这样一来，vDSO代码与`userboot`的偏移量总是固定的。

在编译阶段中，系统调用的入口点符号表会从vDSO ELF映像中提取出来，随后写入到链接脚本的符号定义中。利用每个符号在vDSO映像中相对固定的偏移地址，可在链接脚本提供的`_end`符号的固定偏移量处，定义该符号。通过这种方式，userboot代码可以直接调用到放在内存中，其映像本身之后的，每个确切位置上的vDSO入口点。

相关代码:
> zircon-loader/src/lib.rs
```rust
pub fn run_userboot(images: &Images<impl AsRef<[u8]>>, cmdline: &str) -> Arc<Process> {
    ...
    // vdso
    let vdso_vmo = {
        let elf = ElfFile::new(images.vdso.as_ref()).unwrap();
        let vdso_vmo = VmObject::new_paged(images.vdso.as_ref().len() / PAGE_SIZE + 1);
        vdso_vmo.write(0, images.vdso.as_ref()).unwrap();
        let size = elf.load_segment_size();
        let vmar = vmar
            .allocate_at(
                userboot_size,
                size,
                VmarFlags::CAN_MAP_RXW | VmarFlags::SPECIFIC,
                PAGE_SIZE,
            )
            .unwrap();
        vmar.map_from_elf(&elf, vdso_vmo.clone()).unwrap();
        #[cfg(feature = "std")]
        {
            let offset = elf
                .get_symbol_address("zcore_syscall_entry")
                .expect("failed to locate syscall entry") as usize;
            let syscall_entry = &(kernel_hal_unix::syscall_entry as usize).to_ne_bytes();
            // fill syscall entry x3
            vdso_vmo.write(offset, syscall_entry).unwrap();
            vdso_vmo.write(offset + 8, syscall_entry).unwrap();
            vdso_vmo.write(offset + 16, syscall_entry).unwrap();
        }
        vdso_vmo
    };
    ...

}
```

### bootsvc
bootsvc 通常是usermode加载的第一个程序（与userboot不同，userboot是由内核加载的）。bootsvc提供了几种系统服务：
+ 包含bootfs（/boot）内容的文件系统服务（初始的bootfs映像包含用户空间系统需要运行的所有内容:
  - 可执行文件
  - 共享库和数据文件（包括设备驱动程序或更高级的文件系统的实现）
+ 从bootfs加载的加载程序服务

 
+ bin/component_manager  
+ sh / device_manager    






## 用户程序的组成

> 内核不直接参与用户程序的加载工作（第一个进程除外）
>
> 用户程序强制使用 PIC 和 PIE（位置无关代码）
>
> 内存地址空间组成：Program, Stack, vDSO, Dylibs
>
> 通过 Channel 传递启动信息和句柄











## 系统调用的跳板：vDSO

#### 介绍 vDSO 的作用

vDSO（virtual Dynamic Shared Object），Zircon vDSO 是 Zircon 内核访问系统调用的唯一方法(作为系统调用的跳板)。它之所以是虚拟的，是因为它不是从文件系统中的ELF文件加载的，而是由内核直接提供的vDSO镜像。

<!-- Zircon vDSO是访问Zircon系统调用的唯一手段。vDSO表示虚拟动态共享对象。(动态共享对象是一个术语，用于ELF格式的共享库。)它是虚拟的，因为它不是从文件系统中的ELF文件加载的。相反，vDSO映像由内核直接提供。 -->

> zCore/src/main.rs
```rust
#[cfg(feature = "zircon")]
fn main(ramfs_data: &[u8], cmdline: &str) {
    use zircon_loader::{run_userboot, Images};
    let images = Images::<&[u8]> {
        userboot: include_bytes!("../../prebuilt/zircon/x64/userboot.so"),
        vdso: include_bytes!("../../prebuilt/zircon/x64/libzircon.so"),
        zbi: ramfs_data,
    };
    let _proc = run_userboot(&images, cmdline);
    run();
}
```

它是一个用户态运行的代码，被封装成`prebuilt/zircon/x64/libzircon.so`文件。这个.so 文件装载不是放在文件系统中，而是由内核提供。它被整合在内核image中。

vDSO映像在编译时嵌入到内核中。内核将它作为只读VMO公开给用户空间。内核启动时，会通过计算得到它所在的物理页。当`program loader`设置了一个新进程后，使该进程能够进行系统调用的唯一方法是：`program loader`在新进程的第一个线程开始运行之前，将vDSO映射到新进程的虚拟地址空间（地址随机）。因此，在启动其他能够进行系统调用的进程的每个进程自己本身都必须能够访问vDSO的VMO。

> zircon-loader/src/lib.rs#line167  

```rust
    proc.start(&thread, entry, sp, Some(handle), 0, thread_fn)
        .expect("failed to start main thread");
    proc
```
> zircon-object/src/task/process.rs#line189  

```rust
    thread.start(entry, stack, handle_value as usize, arg2, thread_fn)
```

vDSO被映射到新进程的同时会将映像的`base address`通过`arg2`参数传递给新进程中的第一个线程。通过这个地址，可以在内存中找到ELF的文件头，该文件头指向可用于查找系统调用符号名的其他ELF程序模块。

#### 如何修改 vDSO 源码（libzircon）将 syscall 改为函数调用

##### 有关代码
+ 参考仓库[README.MD](https://github.com/PanQL/zircon/blob/master/README.md)
    > ···解析代码依赖的compile_commands.json将会随build过程生成到**out**文件夹···

##### 如何生成imgs(VDSO,ZBI)
1. clone Zircon代码仓库（从fuchsia官方目录中分离出来的zircon代码）：
    ```shell  
    $ git clone https://github.com/PanQL/zircon.git
    ```
2. 关于Zircon的编译运行  
为了减小仓库体积，我们将prebuilt目录进行了大幅调整;因此运行之前请下载google预编译好的clang，解压后放到某个权限合适的位置，然后在代码的[这个位置](https://github.com/PanQL/zircon/blob/master/public/gn/toolchain/clang.gni#L16)将**绝对目录**修改为对应位置。 
   clang下载链接:
   * [云盘下载链接](https://cloud.tsinghua.edu.cn/d/7ab1d87feecd4b2cb3d8/)  
   * 官方CIPD包下载链接如下  
       * [Linux](https://chrome-infra-packages.appspot.com/p/fuchsia/clang/linux-amd64/+/oEsFSe99FkcDKVxZkAY0MKi6C-yYOan1m-QL45N33W8C)  
       * [Mac](https://chrome-infra-packages.appspot.com/p/fuchsia/clang/mac-amd64/+/Lc64-GTi4kihzkCnW8Vaa80TWTnMpZY0Fy6AqChmqvcC)    


3. 当前只支持在Mac OS及Linux x64上进行编译。  
默认的`make run`和`make build`是针对x64架构的，如果希望编译运行arm架构的zircon，那么需要：
   * 修改out/args.gn中的`legacy-image-x64`为`legacy-image-arm64`  
   * 重新`make build`  
   * `make runarm`  

   



4. 配合zCore中的有关脚本与补丁文件
    - scripts/gen-prebuilt.sh
    - scripts/zircon-libos.patch
   + https://github.com/PanQL/zircon/blob/master/system/ulib/zircon/syscall-entry.h
   + https://github.com/PanQL/zircon/blob/master/system/ulib/zircon/syscalls-x86-64.S
   + zircon-loader/src/lib.rs#line 83-93
```rust
        #[cfg(feature = "std")]
        {
            let offset = elf
                .get_symbol_address("zcore_syscall_entry")
                .expect("failed to locate syscall entry") as usize;
            let syscall_entry = &(kernel_hal_unix::syscall_entry as usize).to_ne_bytes();
            // fill syscall entry x3
            vdso_vmo.write(offset, syscall_entry).unwrap();
            vdso_vmo.write(offset + 8, syscall_entry).unwrap();
            vdso_vmo.write(offset + 16, syscall_entry).unwrap();
        }

```

<!-- 当vsdo 用svc 指令后，这时CPU exception进入内核，到 expections.S 中的 sync_exception 宏（不同ELx， sync_exception的参数不一样）。然后这个 sync_exception 宏中先做一些现场保存的工作， 然后jump到 arm64_syscall_dispatcher 宏。

进入arm64_syscall_dispatcher宏后， 先做一些syscall number检查，然后syscall number 跳到 call_wrapper_table 函数表中相应index项的函数中去（call_wrapper_table 像一个一维的函数指针的数组，syscall number 作index，jump到相应的wrapper syscall function 函数中去）。 -->

#### 加载 vDSO 时修改 vDSO 代码段，填入跳转地址







## 第一个用户程序：userboot

> 实现 zircon-loader 中的 run_userboot 函数
> 
> 能够进入用户态并在第一个系统调用时跳转回来


#### 从`bootfs`加载第一个真正意义上的用户程序。
主要相关代码：
> zircon-loader/src/lib.rs
> zircon-object/src/util/elf_loader.rs

当`userboot`解压完毕`ZBI`中的`bootfs`后，`userboot`将继续从`bootfs`载入程序文件运行。

Zircon中具体的实现流程如下：
1. `userboot`检查从内核接收到的环境字符串，这些字符串代表了一定的内核命令行。
    > zircon-loader/src/main.rs
    ```rust
    #[async_std::main]
    async fn main() {
        kernel_hal_unix::init();
        init_logger();

        let opt = Opt::from_args();
        let images = open_images(&opt.prebuilt_path).expect("failed to read file");

        let proc: Arc<dyn KernelObject> = run_userboot(&images, &opt.cmdline);
        drop(images);

        proc.wait_signal(Signal::USER_SIGNAL_0).await;
    }
    ```
   在Zircon中：
   + 若该字符串内容为```userboot=file```，那么该`file`将作为第一个真正的用户进程加载。
   + 若没有这样的选项，则`userboot`将选择的默认文为`bin/bootsvc`。该文件可在`bootfs`中找到。
  
   而在zCore的实现中：
   + ..
2. 为了加载上述文件，userboot实现了一个功能齐全的ELF程序加载器
   `zircon_object::util::elf_loader::load_from_elf`
    ```rust
        // userboot
        let (entry, userboot_size) = {
            let elf = ElfFile::new(images.userboot.as_ref()).unwrap();
            let size = elf.load_segment_size();
            let vmar = vmar
                .allocate(None, size, VmarFlags::CAN_MAP_RXW, PAGE_SIZE)
                .unwrap();
            vmar.load_from_elf(&elf).unwrap();
            (vmar.addr() + elf.header.pt2.entry_point() as usize, size)
        };
    ```
3. 然后userboot以随机地址加载vDSO。它使用标准约定启动新进程，并给它传递一个channel句柄和vDSO基址。
   `zircon_object::util::elf_loader::map_from_elf`
