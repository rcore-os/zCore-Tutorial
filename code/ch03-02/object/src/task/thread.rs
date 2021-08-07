use {
    super::process::Process,
    super::*,
    crate::object::*,
    alloc::{boxed::Box, sync::Arc},
    bitflags::bitflags,
    core::{
        future::Future,
        ops::Deref,
        pin::Pin,
        task::{Context, Poll, Waker},
    },
    trapframe::{UserContext},
    spin::Mutex,
};

pub use self::thread_state::*;

mod thread_state;

pub struct Thread {
    base: KObjectBase,
    proc: Arc<Process>,
    inner: Mutex<ThreadInner>,
}

impl_kobject!(Thread
    fn related_koid(&self) -> KoID {
        self.proc.id()
    }
);

#[derive(Default)]
struct ThreadInner {
    /// Thread context
    ///
    /// It will be taken away when running this thread.
    context: Option<Box<UserContext>>,
    /// The number of existing `SuspendToken`.
    suspend_count: usize,
    /// The waker of task when suspending.
    waker: Option<Waker>,
    /// Thread state
    ///
    /// NOTE: This variable will never be `Suspended`. On suspended, the
    /// `suspend_count` is non-zero, and this represents the state before suspended.
    state: ThreadState,
    /// Should The ProcessStarting exception generated at start of this thread
    first_thread: bool,
    /// Should The ThreadExiting exception do not block this thread
    killed: bool,
    /// The time this thread has run on cpu
    time: u128,
    flags: ThreadFlag,
}

impl ThreadInner {
    fn state(&self) -> ThreadState {
        // Dying > Exception > Suspend > Blocked
        if self.suspend_count == 0
            || self.context.is_none()
            || self.state == ThreadState::BlockedException
            || self.state == ThreadState::Dying
            || self.state == ThreadState::Dead
        {
            self.state
        } else {
            ThreadState::Suspended
        }
    }

    /// Change state and update signal.
    fn change_state(&mut self, state: ThreadState) {
        self.state = state;
    }
}

bitflags! {
    /// Thread flags.
    #[derive(Default)]
    pub struct ThreadFlag: usize {
        /// The thread currently has a VCPU.
        const VCPU = 1 << 3;
    }
}

/// The type of a new thread function.
pub type ThreadFn = fn(thread: CurrentThread) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

impl Thread {
    /// Create a new thread.
    pub fn create(proc: &Arc<Process>, name: &str) -> ZxResult<Arc<Self>> {
        let thread = Arc::new(Thread {
            base: KObjectBase::with_name(name),
            proc: proc.clone(),
            inner: Mutex::new(ThreadInner {
                context: Some(Box::new(UserContext::default())),
                ..Default::default()
            }),
        });
        proc.add_thread(thread.clone())?;
        Ok(thread)
    }

    /// Get the process.
    pub fn proc(&self) -> &Arc<Process> {
        &self.proc
    }

    /// Start execution on the thread.
    pub fn start(
        self: &Arc<Self>,
        entry: usize,
        stack: usize,
        arg1: usize,
        arg2: usize,
        thread_fn: ThreadFn,
    ) -> ZxResult {
        {
            let mut inner = self.inner.lock();
            let context = inner.context.as_mut().ok_or(ZxError::BAD_STATE)?;
            context.general.rip = entry;
            context.general.rsp = stack;
            context.general.rdi = arg1;
            context.general.rsi = arg2;
            context.general.rflags |= 0x3202;
            inner.change_state(ThreadState::Running);
        }
        kernel_hal::Thread::spawn(thread_fn(CurrentThread(self.clone())), 0);
        Ok(())
    }

    /// Stop the thread. Internal implementation of `exit` and `kill`.
    ///
    /// The thread do not terminate immediately when stopped. It is just made dying.
    /// It will terminate after some cleanups (when `terminate` are called **explicitly** by upper layer).
    fn stop(&self, killed: bool) {
        let mut inner = self.inner.lock();
        if inner.state == ThreadState::Dead {
            return;
        }
        if killed {
            inner.killed = true;
        }
        if inner.state == ThreadState::Dying {
            return;
        }
        inner.change_state(ThreadState::Dying);
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }

    /// Read one aspect of thread state.
    pub fn read_state(&self, kind: ThreadStateKind, buf: &mut [u8]) -> ZxResult<usize> {
        let inner = self.inner.lock();
        let state = inner.state();
        if state != ThreadState::BlockedException && state != ThreadState::Suspended {
            return Err(ZxError::BAD_STATE);
        }
        let context = inner.context.as_ref().ok_or(ZxError::BAD_STATE)?;
        context.read_state(kind, buf)
    }

    /// Write one aspect of thread state.
    pub fn write_state(&self, kind: ThreadStateKind, buf: &[u8]) -> ZxResult {
        let mut inner = self.inner.lock();
        let state = inner.state();
        if state != ThreadState::BlockedException && state != ThreadState::Suspended {
            return Err(ZxError::BAD_STATE);
        }
        let context = inner.context.as_mut().ok_or(ZxError::BAD_STATE)?;
        context.write_state(kind, buf)
    }

    /// Get the thread's information.
    pub fn get_thread_info(&self) -> ThreadInfo {
        let inner = self.inner.lock();
        ThreadInfo {
            state: inner.state() as u32,
        }
    }
    /// Get the thread state.
    pub fn state(&self) -> ThreadState {
        self.inner.lock().state()
    }

    /// Add the parameter to the time this thread has run on cpu.
    pub fn time_add(&self, time: u128) {
        self.inner.lock().time += time;
    }

    /// Get the time this thread has run on cpu.
    pub fn get_time(&self) -> u64 {
        self.inner.lock().time as u64
    }

    /// Set this thread as the first thread of a process.
    pub(super) fn set_first_thread(&self) {
        self.inner.lock().first_thread = true;
    }

    /// Whether this thread is the first thread of a process.
    pub fn is_first_thread(&self) -> bool {
        self.inner.lock().first_thread
    }

    /// Get the thread's flags.
    pub fn flags(&self) -> ThreadFlag {
        self.inner.lock().flags
    }

    /// Apply `f` to the thread's flags.
    pub fn update_flags(&self, f: impl FnOnce(&mut ThreadFlag)) {
        f(&mut self.inner.lock().flags)
    }

    /// Set the thread local fsbase register on x86_64.
    pub fn set_fsbase(&self, fsbase: usize) -> ZxResult {
        let mut inner = self.inner.lock();
        let context = inner.context.as_mut().ok_or(ZxError::BAD_STATE)?;
        context.general.fsbase = fsbase;
        Ok(())
    }

    /// Set the thread local gsbase register on x86_64.
    pub fn set_gsbase(&self, gsbase: usize) -> ZxResult {
        let mut inner = self.inner.lock();
        let context = inner.context.as_mut().ok_or(ZxError::BAD_STATE)?;
        context.general.gsbase = gsbase;
        Ok(())
    }
}

impl Task for Thread {
    fn kill(&self) {
        self.stop(true)
    }

    fn suspend(&self) {
        let mut inner = self.inner.lock();
        inner.suspend_count += 1;
        // let state = inner.state;
        // inner.change_state(state);
    }

    fn resume(&self) {
        let mut inner = self.inner.lock();
        assert_ne!(inner.suspend_count, 0);
        inner.suspend_count -= 1;
        if inner.suspend_count == 0 {
            // let state = inner.state;
            // inner.change_state(state);
            if let Some(waker) = inner.waker.take() {
                waker.wake();
            }
        }
    }
}


/// A handle to current thread.
///
/// This is a wrapper of [`Thread`] that provides additional methods for the thread runner.
/// It can only be obtained from the argument of `thread_fn` in a new thread started by [`Thread::start`].
///
/// It will terminate current thread on drop.
///
/// [`Thread`]: crate::task::Thread
/// [`Thread::start`]: crate::task::Thread::start
pub struct CurrentThread(pub(super) Arc<Thread>);

impl Deref for CurrentThread {
    type Target = Arc<Thread>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for CurrentThread {
    /// Terminate the current running thread.
    fn drop(&mut self) {
        let mut inner = self.inner.lock();
        inner.change_state(ThreadState::Dead);
        self.proc().remove_thread(self.base.id);
    }
}

impl CurrentThread {
    /// Exit the current thread.
    ///
    /// The thread do not terminate immediately when exited. It is just made dying.
    /// It will terminate after some cleanups on this struct drop.
    pub fn exit(&self) {
        self.stop(false);
    }

    /// Wait until the thread is ready to run (not suspended),
    /// and then take away its context to run the thread.
    pub fn wait_for_run(&self) -> impl Future<Output = Box<UserContext>> {
        #[must_use = "wait_for_run does nothing unless polled/`await`-ed"]
        struct RunnableChecker {
            thread: Arc<Thread>,
        }
        impl Future for RunnableChecker {
            type Output = Box<UserContext>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                let mut inner = self.thread.inner.lock();
                if inner.state() != ThreadState::Suspended {
                    // resume:  return the context token from thread object
                    // There is no need to call change_state here
                    // since take away the context of a non-suspended thread won't change it's state
                    Poll::Ready(inner.context.take().unwrap())
                } else {
                    // suspend: put waker into the thread object
                    inner.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
        RunnableChecker {
            thread: self.0.clone(),
        }
    }

    /// The thread ends running and takes back the context.
    pub fn end_running(&self, context: Box<UserContext>) {
        let mut inner = self.inner.lock();
        inner.context = Some(context);
        // let state = inner.state;
        // inner.change_state(state);
    }

    /// Access saved context of current thread.
    ///
    /// Will panic if the context is not availiable.
    pub fn with_context<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&mut UserContext) -> T,
    {
        let mut inner = self.inner.lock();
        let mut cx = inner.context.as_mut().unwrap();
        f(&mut cx)
    }
}

/// The thread state.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ThreadState {
    /// The thread has been created but it has not started running yet.
    New = 0,
    /// The thread is running user code normally.
    Running = 1,
    /// Stopped due to `zx_task_suspend()`.
    Suspended = 2,
    /// In a syscall or handling an exception.
    Blocked = 3,
    /// The thread is in the process of being terminated, but it has not been stopped yet.
    Dying = 4,
    /// The thread has stopped running.
    Dead = 5,
    /// The thread is stopped in an exception.
    BlockedException = 0x103,
    /// The thread is stopped in `zx_nanosleep()`.
    BlockedSleeping = 0x203,
    /// The thread is stopped in `zx_futex_wait()`.
    BlockedFutex = 0x303,
    /// The thread is stopped in `zx_port_wait()`.
    BlockedPort = 0x403,
    /// The thread is stopped in `zx_channel_call()`.
    BlockedChannel = 0x503,
    /// The thread is stopped in `zx_object_wait_one()`.
    BlockedWaitOne = 0x603,
    /// The thread is stopped in `zx_object_wait_many()`.
    BlockedWaitMany = 0x703,
    /// The thread is stopped in `zx_interrupt_wait()`.
    BlockedInterrupt = 0x803,
    /// Pager.
    BlockedPager = 0x903,
}

impl Default for ThreadState {
    fn default() -> Self {
        ThreadState::New
    }
}

/// The thread information.
#[repr(C)]
pub struct ThreadInfo {
    state: u32,
}

#[cfg(test)]
mod tests {
    use super::job::Job;
    use super::*;
    use kernel_hal::timer_now;
    use kernel_hal::GeneralRegs;
    use core::time::Duration;

    #[test]
    fn create() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");
        assert_eq!(thread.flags(), ThreadFlag::empty());

        assert_eq!(thread.related_koid(), proc.id());
        let child = proc.get_child(thread.id()).unwrap().downcast_arc().unwrap();
        assert!(Arc::ptr_eq(&child, &thread));
    }

    #[async_std::test]
    async fn start() {
        kernel_hal_unix::init();
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");
        let thread1 = Thread::create(&proc, "thread1").expect("failed to create thread");

        // function for new thread
        async fn new_thread(thread: CurrentThread) {
            let cx = thread.wait_for_run().await;
            assert_eq!(cx.general.rip, 1);
            assert_eq!(cx.general.rsp, 4);
            assert_eq!(cx.general.rdi, 3);
            assert_eq!(cx.general.rsi, 2);
            async_std::task::sleep(Duration::from_millis(10)).await;
            thread.end_running(cx);
        }

        // start a new thread
        let handle = Handle::new(proc.clone(), Rights::DEFAULT_PROCESS);
        proc.start(&thread, 1, 4, Some(handle.clone()), 2, |thread| {
            Box::pin(new_thread(thread))
        })
        .expect("failed to start thread");

        // check info and state
        let info = proc.get_info();
        assert!(info.started && !info.has_exited && info.return_code == 0);
        assert_eq!(proc.status(), Status::Running);
        assert_eq!(thread.state(), ThreadState::Running);

        // start again should fail
        assert_eq!(
            proc.start(&thread, 1, 4, Some(handle.clone()), 2, |thread| Box::pin(
                new_thread(thread)
            )),
            Err(ZxError::BAD_STATE)
        );

        // start another thread should fail
        assert_eq!(
            proc.start(&thread1, 1, 4, Some(handle.clone()), 2, |thread| Box::pin(
                new_thread(thread)
            )),
            Err(ZxError::BAD_STATE)
        );

        // wait 100ms for the new thread to exit
        async_std::task::sleep(core::time::Duration::from_millis(100)).await;

        // no other references to `Thread`
        assert_eq!(Arc::strong_count(&thread), 1);
        assert_eq!(thread.state(), ThreadState::Dead);
    }


    #[test]
    fn info() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        let info = thread.get_thread_info();
        assert!(info.state == thread.state() as u32);
    }

    #[test]
    fn read_write_state() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        const SIZE: usize = core::mem::size_of::<GeneralRegs>();
        let mut buf = [0; 10];
        assert_eq!(
            thread.read_state(ThreadStateKind::General, &mut buf).err(),
            Some(ZxError::BAD_STATE)
        );
        assert_eq!(
            thread.write_state(ThreadStateKind::General, &buf).err(),
            Some(ZxError::BAD_STATE)
        );

        thread.suspend();

        assert_eq!(
            thread.read_state(ThreadStateKind::General, &mut buf).err(),
            Some(ZxError::BUFFER_TOO_SMALL)
        );
        assert_eq!(
            thread.write_state(ThreadStateKind::General, &buf).err(),
            Some(ZxError::BUFFER_TOO_SMALL)
        );

        let mut buf = [0; SIZE];
        assert!(thread
            .read_state(ThreadStateKind::General, &mut buf)
            .is_ok());
        assert!(thread.write_state(ThreadStateKind::General, &buf).is_ok());
        // TODO
    }

    #[async_std::test]
    async fn wait_for_run() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        assert_eq!(thread.state(), ThreadState::New);

        thread
            .start(0, 0, 0, 0, |thread| Box::pin(new_thread(thread)))
            .unwrap();
        async fn new_thread(thread: CurrentThread) {
            assert_eq!(thread.state(), ThreadState::Running);

            // without suspend
            let context = thread.wait_for_run().await;
            thread.end_running(context);

            // with suspend
            thread.suspend();
            thread.suspend();
            assert_eq!(thread.state(), ThreadState::Suspended);
            async_std::task::spawn({
                let thread = (*thread).clone();
                async move {
                    async_std::task::sleep(Duration::from_millis(10)).await;
                    thread.resume();
                    async_std::task::sleep(Duration::from_millis(10)).await;
                    thread.resume();
                }
            });
            let time = timer_now();
            let _context = thread.wait_for_run().await;
            assert!(timer_now() - time >= Duration::from_millis(20));
        }
        // FIX ME
        // let thread: Arc<dyn KernelObject> = thread;
        // thread.wait_signal(Signal::THREAD_TERMINATED).await;
    }

    #[test]
    fn time() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        assert_eq!(thread.get_time(), 0);
        thread.time_add(10);
        assert_eq!(thread.get_time(), 10);
    }
}
