use super::*;

mod job;
mod job_policy;
mod process;

/// Task (Thread, Process, or Job)
pub trait Task: Sync + Send {
    /// Kill the task. The task do not terminate immediately when killed.
    /// It will terminate after all its children are terminated or some cleanups are finished.
    fn kill(&self);

    /// Suspend the task. Currently only thread or process handles may be suspended.
    fn suspend(&self);

    /// Resume the task
    fn resume(&self);

    /// Get the exceptionate.
    fn exceptionate(&self) -> Arc<Exceptionate>;

    /// Get the debug exceptionate.
    fn debug_exceptionate(&self) -> Arc<Exceptionate>;
}

/// The return code set when a task is killed via zx_task_kill().
pub const TASK_RETCODE_SYSCALL_KILL: i64 = -1028;
