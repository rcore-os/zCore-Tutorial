# 用户程序

zCore采用的是微内核的设计风格。微内核设计的一个复杂问题是”如何引导初始用户空间进程“。通常这是通过让内核实现最小版本的文件系统读取和程序加载来实现的引导。