# Zircon 内存管理模型
Zircon 中有两个跟内存管理有关的对象 VMO（Virtual Memory Object）和 VMAR （Virtual Memory Address Region）。VMO 主要负责管理物理内存页面，VMAR 主要负责进程虚拟内存管理。当创建一个进程的时候，需要使用到内存的时候，都需要创建一个 VMO，然后将这个 VMO map 到 VMAR上面。
