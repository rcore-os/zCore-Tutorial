use super::*;

mod job;
mod job_policy;
mod process;
mod thread;

pub use {self::job::*, self::job_policy::*, self::process::*, self::thread::*};

/// Task (Thread, Process, or Job)
pub trait Task: Sync + Send {
    /// Kill the task. The task do not terminate immediately when killed.
    /// It will terminate after all its children are terminated or some cleanups are finished.
    fn kill(&self);

    /// Suspend the task. Currently only thread or process handles may be suspended.
    fn suspend(&self);

    /// Resume the task
    fn resume(&self);
}

/// The return code set when a task is killed via zx_task_kill().
pub const TASK_RETCODE_SYSCALL_KILL: i64 = -1028;
