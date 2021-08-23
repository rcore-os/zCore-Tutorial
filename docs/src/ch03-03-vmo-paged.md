# 物理内存：按页分配的 VMO

## 简介

> 说明一下：Zircon 的官方实现中为了高效支持写时复制，使用了复杂精巧的树状数据结构，但它同时也引入了复杂性和各种 Bug。
> 我们在这里只实现一个简单版本，完整实现留给读者自行探索。
>
> 介绍 commit 操作的意义和作用

commit_page 和 commit_pages_with 函数的作用：用于检查物理页帧是否已经分配。

## HAL：物理内存管理

> 在 HAL 中实现 PhysFrame 和最简单的分配器

### kernel-hal
```rust
#[repr(C)]
pub struct PhysFrame {
		// paddr 物理地址
    paddr: PhysAddr,
}

impl PhysFrame {
		// 分配物理页帧
    #[linkage = "weak"]
    #[export_name = "hal_frame_alloc"]
    pub fn alloc() -> Option<Self> {
        unimplemented!()
    }

    #[linkage = "weak"]
    #[export_name = "hal_frame_alloc_contiguous"]
    pub fn alloc_contiguous_base(_size: usize, _align_log2: usize) -> Option<PhysAddr> {
        unimplemented!()
    }

    pub fn alloc_contiguous(size: usize, align_log2: usize) -> Vec<Self> {
        PhysFrame::alloc_contiguous_base(size, align_log2).map_or(Vec::new(), |base| {
            (0..size)
                .map(|i| PhysFrame {
                    paddr: base + i * PAGE_SIZE,
                })
                .collect()
        })
    }

    pub fn alloc_zeroed() -> Option<Self> {
        Self::alloc().map(|f| {
            pmem_zero(f.addr(), PAGE_SIZE);
            f
        })
    }

    pub fn alloc_contiguous_zeroed(size: usize, align_log2: usize) -> Vec<Self> {
        PhysFrame::alloc_contiguous_base(size, align_log2).map_or(Vec::new(), |base| {
            pmem_zero(base, size * PAGE_SIZE);
            (0..size)
                .map(|i| PhysFrame {
                    paddr: base + i * PAGE_SIZE,
                })
                .collect()
        })
    }

    pub fn addr(&self) -> PhysAddr {
        self.paddr
    }

    #[linkage = "weak"]
    #[export_name = "hal_zero_frame_paddr"]
    pub fn zero_frame_addr() -> PhysAddr {
        unimplemented!()
    }
}

impl Drop for PhysFrame {
    #[linkage = "weak"]
    #[export_name = "hal_frame_dealloc"]
    fn drop(&mut self) {
        unimplemented!()
    }
}
```
### kernel-hal-unix
通过下面的代码可以构造一个页帧号。`(PAGE_SIZE..PMEM_SIZE).step_by(PAGE_SIZE).collect()` 可以每隔 PAGE_SIZE 生成一个页帧的开始位置。
```rust
lazy_static! {
    static ref AVAILABLE_FRAMES: Mutex<VecDeque<usize>> =
        Mutex::new((PAGE_SIZE..PMEM_SIZE).step_by(PAGE_SIZE).collect());
}
```
分配一块物理页帧就是从 AVAILABLE_FRAMES 中通过 pop_front 弹出一个页号
```rust
impl PhysFrame {
    #[export_name = "hal_frame_alloc"]
    pub fn alloc() -> Option<Self> {
        let ret = AVAILABLE_FRAMES
            .lock()
            .unwrap()
            .pop_front()
            .map(|paddr| PhysFrame { paddr });
        trace!("frame alloc: {:?}", ret);
        ret
    }
    #[export_name = "hal_zero_frame_paddr"]
    pub fn zero_frame_addr() -> PhysAddr {
        0
    }
}

impl Drop for PhysFrame {
    #[export_name = "hal_frame_dealloc"]
    fn drop(&mut self) {
        trace!("frame dealloc: {:?}", self);
        AVAILABLE_FRAMES.lock().unwrap().push_back(self.paddr);
    }
}
```
## 辅助结构：BlockRange 迭代器

> 实现 BlockRange

在按页分配内存的 VMObjectPaged 的读和写的方法中会使用到一个 BlockIter 迭代器。BlockIter 主要用于将一段内存分块，每次返回这一块的信息也就是 BlockRange。
### BlockIter
```rust
#[derive(Debug, Eq, PartialEq)]
pub struct BlockRange {
    pub block: usize,
    pub begin: usize, // 块内地址开始位置
    pub end: usize, // 块内地址结束位置
    pub block_size_log2: u8,
}

/// Given a range and iterate sub-range for each block
pub struct BlockIter {
    pub begin: usize,
    pub end: usize,
    pub block_size_log2: u8,
}
```
block_size_log2 是 log 以2为底 block size, 比如：block size 大小为4096，则 block_size_log2 为 12。block 是块编号。
```rust
impl BlockRange {
    pub fn len(&self) -> usize {
        self.end - self.begin
    }
    pub fn is_full(&self) -> bool {
        self.len() == (1usize << self.block_size_log2)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn origin_begin(&self) -> usize {
        (self.block << self.block_size_log2) + self.begin
    }
    pub fn origin_end(&self) -> usize {
        (self.block << self.block_size_log2) + self.end
    }
}

impl Iterator for BlockIter {
    type Item = BlockRange;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.begin >= self.end {
            return None;
        }
        let block_size_log2 = self.block_size_log2;
        let block_size = 1usize << self.block_size_log2;
        let block = self.begin / block_size;
        let begin = self.begin % block_size;
		// 只有最后一块需要计算块内最后的地址，其他的直接返回块的大小
        let end = if block == self.end / block_size {
            self.end % block_size
        } else {
            block_size
        };
        self.begin += end - begin;
        Some(BlockRange {
            block,
            begin,
            end,
            block_size_log2,
        })
    }
}
```
## 实现按页分配的 VMO

> 实现 for_each_page, commit, read, write 函数

按页分配的 VMO 结构体如下：
```rust
pub struct VMObjectPaged {
    inner: Mutex<VMObjectPagedInner>,
}

/// The mutable part of `VMObjectPaged`.
#[derive(Default)]
struct VMObjectPagedInner {
    /// Physical frames of this VMO.
    frames: Vec<PhysFrame>,
    /// Cache Policy
    cache_policy: CachePolicy,
    /// Is contiguous
    contiguous: bool,
    /// Sum of pin_count
    pin_count: usize,
    /// All mappings to this VMO.
    mappings: Vec<Weak<VmMapping>>,
}
```
VMObjectPage 有两个 new 方法
```rust
impl VMObjectPaged {
    /// Create a new VMO backing on physical memory allocated in pages.
    pub fn new(pages: usize) -> Arc<Self> {
        let mut frames = Vec::new();
        frames.resize_with(pages, || PhysFrame::alloc_zeroed().unwrap()); // 分配 pages 个页帧号，并将这些页帧号的内存清零
        Arc::new(VMObjectPaged {
            inner: Mutex::new(VMObjectPagedInner {
                frames,
                ..Default::default()
            }),
        })
    }

    /// Create a list of contiguous pages
    pub fn new_contiguous(pages: usize, align_log2: usize) -> ZxResult<Arc<Self>> {
        let frames = PhysFrame::alloc_contiguous_zeroed(pages, align_log2 - PAGE_SIZE_LOG2);
        if frames.is_empty() {
            return Err(ZxError::NO_MEMORY);
        }
        Ok(Arc::new(VMObjectPaged {
            inner: Mutex::new(VMObjectPagedInner {
                frames,
                contiguous: true,
                ..Default::default()
            }),
        }))
    }
}
```
VMObjectPaged 的读和写用到了一个非常重要的函数 for_each_page 。首先它先构造了一个 BlockIter 迭代器，然后调用传入的函数进行读或者写。
```rust
impl VMObjectPagedInner {
		/// Helper function to split range into sub-ranges within pages.
    ///
    /// ```text
    /// VMO range:
    /// |----|----|----|----|----|
    ///
    /// buf:
    ///            [====len====]
    /// |--offset--|
    ///
    /// sub-ranges:
    ///            [===]
    ///                [====]
    ///                     [==]
    /// ```
    ///
    /// `f` is a function to process in-page ranges.
    /// It takes 2 arguments:
    /// * `paddr`: the start physical address of the in-page range.
    /// * `buf_range`: the range in view of the input buffer.
		fn for_each_page(
        &mut self,
        offset: usize,
        buf_len: usize,
        mut f: impl FnMut(PhysAddr, Range<usize>),
    ) {
        let iter = BlockIter {
            begin: offset,
            end: offset + buf_len,
            block_size_log2: 12,
        };
        for block in iter {
						// 获取这一块开始的物理地址
            let paddr = self.frames[block.block].addr();
						// 这块物理地址的范围
            let buf_range = block.origin_begin() - offset..block.origin_end() - offset;
            f(paddr + block.begin, buf_range);
        }  
    }
}
```
read 和 write 函数，一个传入的是 `kernel_hal::pmem_read` ，另外一个是 `kernel_hal::pmem_write`
```rust
impl VMObjectTrait for VMObjectPaged {
    fn read(&self, offset: usize, buf: &mut [u8]) -> ZxResult {
        let mut inner = self.inner.lock();
        if inner.cache_policy != CachePolicy::Cached {
            return Err(ZxError::BAD_STATE);
        }
        inner.for_each_page(offset, buf.len(), |paddr, buf_range| {
            kernel_hal::pmem_read(paddr, &mut buf[buf_range]);
        });
        Ok(())
    }

    fn write(&self, offset: usize, buf: &[u8]) -> ZxResult {
        let mut inner = self.inner.lock();
        if inner.cache_policy != CachePolicy::Cached {
            return Err(ZxError::BAD_STATE);
        }
        inner.for_each_page(offset, buf.len(), |paddr, buf_range| {
            kernel_hal::pmem_write(paddr, &buf[buf_range]);
        });
        Ok(())
    }
}
```
commit 函数
```rust
impl VMObjectTrait for VMObjectPaged {
		fn commit_page(&self, page_idx: usize, _flags: MMUFlags) -> ZxResult<PhysAddr> {
        let inner = self.inner.lock();
        Ok(inner.frames[page_idx].addr())
    }

    fn commit_pages_with(
        &self,
        f: &mut dyn FnMut(&mut dyn FnMut(usize, MMUFlags) -> ZxResult<PhysAddr>) -> ZxResult,
    ) -> ZxResult {
        let inner = self.inner.lock();
        f(&mut |page_idx, _| Ok(inner.frames[page_idx].addr()))
    }
}
```
## VMO 复制

> 实现 create_child 函数

create_child 是将原 VMObjectPaged 的内容拷贝一份
```rust
// object/vm/vmo/paged.rs

impl VMObjectTrait for VMObjectPaged {
		fn create_child(&self, offset: usize, len: usize) -> ZxResult<Arc<dyn VMObjectTrait>> {
        assert!(page_aligned(offset));
        assert!(page_aligned(len));
        let mut inner = self.inner.lock();
        let child = inner.create_child(offset, len)?;
        Ok(child)
    }

		/// Create a snapshot child VMO.
    fn create_child(&mut self, offset: usize, len: usize) -> ZxResult<Arc<VMObjectPaged>> {
        // clone contiguous vmo is no longer permitted
        // https://fuchsia.googlesource.com/fuchsia/+/e6b4c6751bbdc9ed2795e81b8211ea294f139a45
        if self.contiguous {
            return Err(ZxError::INVALID_ARGS);
        }
        if self.cache_policy != CachePolicy::Cached || self.pin_count != 0 {
            return Err(ZxError::BAD_STATE);
        }
        let mut frames = Vec::with_capacity(pages(len));
        for _ in 0..pages(len) {
            frames.push(PhysFrame::alloc().ok_or(ZxError::NO_MEMORY)?);
        }
        for (i, frame) in frames.iter().enumerate() {
            if let Some(src_frame) = self.frames.get(pages(offset) + i) {
                kernel_hal::frame_copy(src_frame.addr(), frame.addr())
            } else {
                kernel_hal::pmem_zero(frame.addr(), PAGE_SIZE);
            }
        }
        // create child VMO
        let child = Arc::new(VMObjectPaged {
            inner: Mutex::new(VMObjectPagedInner {
                frames,
                ..Default::default()
            }),
        });
        Ok(child)
    }
}

// kernel-hal-unix/sr/lib.rs

/// Copy content of `src` frame to `target` frame
#[export_name = "hal_frame_copy"]
pub fn frame_copy(src: PhysAddr, target: PhysAddr) {
    trace!("frame_copy: {:#x} <- {:#x}", target, src);
    assert!(src + PAGE_SIZE <= PMEM_SIZE && target + PAGE_SIZE <= PMEM_SIZE);
    ensure_mmap_pmem();
    unsafe {
        let buf = phys_to_virt(src) as *const u8;
        buf.copy_to_nonoverlapping(phys_to_virt(target) as _, PAGE_SIZE);
    }
}

/// Zero physical memory at `[paddr, paddr + len)`
#[export_name = "hal_pmem_zero"]
pub fn pmem_zero(paddr: PhysAddr, len: usize) {
    trace!("pmem_zero: addr={:#x}, len={:#x}", paddr, len);
    assert!(paddr + len <= PMEM_SIZE);
    ensure_mmap_pmem();
    unsafe {
        core::ptr::write_bytes(phys_to_virt(paddr) as *mut u8, 0, len);
    }
}
```
