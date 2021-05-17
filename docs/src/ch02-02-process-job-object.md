# 进程管理：Process 与 Job 对象



> 介绍 Process 与 Job 的整体设计
>
> 实现 Process 和 Job 对象的基本框架，支持树状结构
>

## 作业Job
### 概要
作业是一组进程，可能还包括其他（子）作业。作业用于跟踪执行内核操作的特权（即使用各种选项进行各种syscall），以及跟踪和限制基本资源（例如内存，CPU）的消耗。每个进程都属于一个作业。作业也可以嵌套，并且除根作业外的每个作业都属于一个（父）作业。
### 描述

作业是包含以下内容的对象：

- 对父作业的引用
- 一组子作业（每个子作业的父作业既是这个作业）
- 一组成员进程
- 一套策略（Policy）

由多个进程组成的“应用程序”可作为单个实体，被作业基于一套策略进行控制。

### 作业策略Job Policy 

[策略policy](https://fuchsia.dev/fuchsia-src/concepts/settings/policy/policy_concepts?hl=en) 可在Kernel运行时动态修改系统的各种配置（setting）。作业策略主要涉及作业安全性和资源使用的条件（Condition）限制。

#### 策略的行为PolicyAction

策略的行为包括：

- Allow 允许条件
- Deny 拒绝条件
- AllowException 通过 debugt port 生成异常，异常处理完毕后可恢复执行且运行条件
- DenyException 通过 debugt port 生成异常，异常处理完毕后可恢复执行
- Kill 杀死进程

#### 应用策略时的条件 PolicyCondition

应用策略时的条件包括：

- BadHandle： 此作业下的某个进程正在尝试发出带有无效句柄的syscall。在这种情况下，`PolicyAction::Allow`并且`PolicyAction::Deny`是等效的：如果syscall返回，它将始终返回错误ZX_ERR_BAD_HANDLE。
- WrongObject：此作业下的某个进程正在尝试发出带有不支持该操作的句柄的syscall。
- VmarWx：此作业下的进程正在尝试映射具有写执行访问权限的地址区域。
- NewAny：代表上述所有ZX_NEW条件的特殊条件，例如NEW_VMO，NEW_CHANNEL，NEW_EVENT，NEW_EVENTPAIR，NEW_PORT，NEW_SOCKET，NEW_FIFO和任何将来的ZX_NEW策略。这将包括不需要父对象来创建的所有新内核对象。
- NewVMO：此作业下的某个进程正在尝试创建新的vm对象。
- NewChannel：此作业下的某个进程正在尝试创建新通道。
- NewEvent：此作业下的一个进程正在尝试创建一个新事件。
- NewEventPair：此作业下的某个进程正在尝试创建新的事件对。
- NewPort：此作业下的进程正在尝试创建新端口。
- NewSocket：此作业下的进程正在尝试创建新的套接字。
- NewFIFO：此工作下的一个进程正在尝试创建一个新的FIFO。
- NewTimer：此作业下的某个进程正在尝试创建新的计时器。
- NewProcess：此作业下的进程正在尝试创建新进程。
- NewProfile：此作业下的一个进程正在尝试创建新的配置文件。
- AmbientMarkVMOExec：此作业下的某个进程正在尝试使用带有ZX_HANDLE_INVALID的zx_vmo_replace_as_executable（）作为第二个参数，而不是有效的ZX_RSRC_KIND_VMEX。

## 进程Process