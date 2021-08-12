use {
    super::{job::Job, job_policy::*, thread::*, *},
    crate::{error::*, object::*, vm::*},
    alloc::{sync::Arc, vec::Vec},
    core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
    hashbrown::HashMap,
    spin::Mutex,
};

pub struct Process {
    base: KObjectBase,
    job: Arc<Job>,
    policy: JobPolicy,
    vmar: Arc<VmAddressRegion>,
    inner: Mutex<ProcessInner>,
}

impl_kobject!(Process
    fn get_child(&self, id: KoID) -> ZxResult<Arc<dyn KernelObject>> {
        let inner = self.inner.lock();
        let thread = inner.threads.iter().find(|o| o.id() == id).ok_or(ZxError::NOT_FOUND)?;
        Ok(thread.clone())
    }
    fn related_koid(&self) -> KoID {
        self.job.id()
    }
);

#[derive(Default)]
struct ProcessInner {
    max_handle_id: u32,
    status: Status,
    handles: HashMap<HandleValue, Handle>,
    threads: Vec<Arc<Thread>>,
}

/// Status of a process.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Status {
    /// Initial state, no thread present in process.
    Init,
    /// First thread has started and is running.
    Running,
    /// Process has exited with the code.
    Exited(i64),
}

impl Default for Status {
    fn default() -> Self {
        Status::Init
    }
}

impl Process {
    /// Create a new process in the `job`.
    pub fn create(job: &Arc<Job>, name: &str) -> ZxResult<Arc<Self>> {
        let proc = Arc::new(Process {
            base: KObjectBase::with_name(name),
            job: job.clone(),
            policy: job.policy(),
            vmar: VmAddressRegion::new_root(),
            inner: Mutex::new(ProcessInner::default()),
        });
        job.add_process(proc.clone())?;
        Ok(proc)
    }

    /// Get a handle from the process
    fn get_handle(&self, handle_value: HandleValue) -> ZxResult<Handle> {
        self.inner.lock().get_handle(handle_value)
    }

    /// 添加一个新的对象句柄
    pub fn add_handle(&self, handle: Handle) -> HandleValue {
        self.inner.lock().add_handle(handle)
    }

    /// 删除一个对象句柄
    pub fn remove_handle(&self, handle_value: HandleValue) -> ZxResult<Handle> {
        self.inner.lock().remove_handle(handle_value)
    }

    /// Add all handles to the process
    pub fn add_handles(&self, handles: Vec<Handle>) -> Vec<HandleValue> {
        let mut inner = self.inner.lock();
        handles.into_iter().map(|h| inner.add_handle(h)).collect()
    }

    /// Remove all handles from the process.
    pub fn remove_handles(&self, handle_values: &[HandleValue]) -> ZxResult<Vec<Handle>> {
        let mut inner = self.inner.lock();
        handle_values
            .iter()
            .map(|h| inner.remove_handle(*h))
            .collect()
    }

    /// Get the kernel object corresponding to this `handle_value`
    pub fn get_object<T: KernelObject>(&self, handle_value: HandleValue) -> ZxResult<Arc<T>> {
        let handle = self.get_handle(handle_value)?;
        let object = handle
            .object
            .downcast_arc::<T>()
            .map_err(|_| ZxError::WRONG_TYPE)?;
        Ok(object)
    }

    /// 根据句柄值查找内核对象，并检查权限
    pub fn get_object_with_rights<T: KernelObject>(
        &self,
        handle_value: HandleValue,
        desired_rights: Rights,
    ) -> ZxResult<Arc<T>> {
        let handle = self.get_handle(handle_value)?;
        // check type before rights
        let object = handle
            .object
            .downcast_arc::<T>()
            .map_err(|_| ZxError::WRONG_TYPE)?;
        if !handle.rights.contains(desired_rights) {
            return Err(ZxError::ACCESS_DENIED);
        }
        Ok(object)
    }

    /// Get the kernel object corresponding to this `handle_value` and this handle's rights.
    pub fn get_object_and_rights<T: KernelObject>(
        &self,
        handle_value: HandleValue,
    ) -> ZxResult<(Arc<T>, Rights)> {
        let handle = self.get_handle(handle_value)?;
        let object = handle
            .object
            .downcast_arc::<T>()
            .map_err(|_| ZxError::WRONG_TYPE)?;
        Ok((object, handle.rights))
    }

    /// Remove a handle referring to a kernel object of the given type from the process.
    pub fn remove_object<T: KernelObject>(&self, handle_value: HandleValue) -> ZxResult<Arc<T>> {
        let handle = self.remove_handle(handle_value)?;
        let object = handle
            .object
            .downcast_arc::<T>()
            .map_err(|_| ZxError::WRONG_TYPE)?;
        Ok(object)
    }

    pub fn start(
        &self,
        thread: &Arc<Thread>,
        entry: usize,
        stack: usize,
        arg1: Option<Handle>,
        arg2: usize,
        thread_fn: ThreadFn,
    ) -> ZxResult {
        let handle_value;
        {
            let mut inner = self.inner.lock();
            if !inner.contains_thread(thread) {
                return Err(ZxError::ACCESS_DENIED);
            }
            if inner.status != Status::Init {
                return Err(ZxError::BAD_STATE);
            }
            inner.status = Status::Running;
            handle_value = arg1.map_or(INVALID_HANDLE, |handle| inner.add_handle(handle));
        }
        thread.set_first_thread();
        match thread.start(entry, stack, handle_value as usize, arg2, thread_fn) {
            Ok(_) => Ok(()),
            Err(err) => {
                let mut inner = self.inner.lock();
                if handle_value != INVALID_HANDLE {
                    inner.remove_handle(handle_value).ok();
                }
                Err(err)
            }
        }
    }

    /// Exit current process with `retcode`.
    /// The process do not terminate immediately when exited.
    /// It will terminate after all its child threads are terminated.
    pub fn exit(&self, retcode: i64) {
        let mut inner = self.inner.lock();
        if let Status::Exited(_) = inner.status {
            return;
        }
        inner.status = Status::Exited(retcode);
        if inner.threads.is_empty() {
            inner.handles.clear();
            drop(inner);
            self.terminate();
            return;
        }
        for thread in inner.threads.iter() {
            thread.kill();
        }
        inner.handles.clear();
    }

    /// The process finally terminates.
    fn terminate(&self) {
        let mut inner = self.inner.lock();
        let _retcode = match inner.status {
            Status::Exited(retcode) => retcode,
            _ => {
                inner.status = Status::Exited(0);
                0
            }
        };
        self.job.remove_process(self.base.id);
    }

    /// Check whether `condition` is allowed in the parent job's policy.
    pub fn check_policy(&self, condition: PolicyCondition) -> ZxResult {
        match self
            .policy
            .get_action(condition)
            .unwrap_or(PolicyAction::Allow)
        {
            PolicyAction::Allow => Ok(()),
            PolicyAction::Deny => Err(ZxError::ACCESS_DENIED),
            _ => unimplemented!(),
        }
    }

    /// Get process status.
    pub fn status(&self) -> Status {
        self.inner.lock().status
    }

    /// Get the `VmAddressRegion` of the process.
    pub fn vmar(&self) -> Arc<VmAddressRegion> {
        self.vmar.clone()
    }

    /// Get the job of the process.
    pub fn job(&self) -> Arc<Job> {
        self.job.clone()
    }

    /// Add a thread to the process.
    pub(super) fn add_thread(&self, thread: Arc<Thread>) -> ZxResult {
        let mut inner = self.inner.lock();
        if let Status::Exited(_) = inner.status {
            return Err(ZxError::BAD_STATE);
        }
        inner.threads.push(thread);
        Ok(())
    }

    /// Remove a thread from the process.
    ///
    /// If no more threads left, exit the process.
    pub(super) fn remove_thread(&self, tid: KoID) {
        let mut inner = self.inner.lock();
        inner.threads.retain(|t| t.id() != tid);
        if inner.threads.is_empty() {
            drop(inner);
            self.terminate();
        }
    }

    /// Get KoIDs of Threads.
    pub fn thread_ids(&self) -> Vec<KoID> {
        self.inner.lock().threads.iter().map(|t| t.id()).collect()
    }

    /// Get information of this process.
    pub fn get_info(&self) -> ProcessInfo {
        let mut info = ProcessInfo {
            ..Default::default()
        };
        match self.inner.lock().status {
            Status::Init => {
                info.started = false;
                info.has_exited = false;
            }
            Status::Running => {
                info.started = true;
                info.has_exited = false;
            }
            Status::Exited(ret) => {
                info.return_code = ret;
                info.has_exited = true;
                info.started = true;
            }
        }
        info
    }
}

impl Process {
    pub fn wait_for_end(self: Arc<Self>) -> impl Future<Output = i64> {
        struct ProcessEndFuture {
            proc: Arc<Process>,
        }
        impl Future for ProcessEndFuture {
            type Output = i64;

            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                if let Status::Exited(exit_code) = self.proc.status() {
                    Poll::Ready(exit_code)
                } else {
                    let waker = cx.waker().clone();
                    waker.wake_by_ref();
                    Poll::Pending
                }
            }
        }
        ProcessEndFuture {
            proc: Arc::clone(&self),
        }
    }
}

/// Information of a process.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Default)]
pub struct ProcessInfo {
    pub return_code: i64,
    pub started: bool,
    pub has_exited: bool,
}

impl Task for Process {
    fn kill(&self) {
        self.exit(TASK_RETCODE_SYSCALL_KILL);
    }

    fn suspend(&self) {
        let inner = self.inner.lock();
        for thread in inner.threads.iter() {
            thread.suspend();
        }
    }

    fn resume(&self) {
        let inner = self.inner.lock();
        for thread in inner.threads.iter() {
            thread.resume();
        }
    }
}

impl ProcessInner {
    /// Add a handle to the process
    fn add_handle(&mut self, handle: Handle) -> HandleValue {
        let key = (self.max_handle_id << 2) | 0x3u32;
        self.max_handle_id += 1;
        self.handles.insert(key, handle);
        key
    }

    fn remove_handle(&mut self, handle_value: HandleValue) -> ZxResult<Handle> {
        let handle = self
            .handles
            .remove(&handle_value)
            .ok_or(ZxError::BAD_HANDLE)?;
        Ok(handle)
    }

    fn get_handle(&mut self, handle_value: HandleValue) -> ZxResult<Handle> {
        let handle = self.handles.get(&handle_value).ok_or(ZxError::BAD_HANDLE)?;
        Ok(handle.clone())
    }

    /// Whether `thread` is in this process.
    fn contains_thread(&self, thread: &Arc<Thread>) -> bool {
        self.threads.iter().any(|t| Arc::ptr_eq(t, thread))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");

        assert_eq!(proc.related_koid(), root_job.id());
        assert!(Arc::ptr_eq(&root_job, &proc.job()));
    }

    #[test]
    fn handle() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let handle = Handle::new(proc.clone(), Rights::DEFAULT_PROCESS);

        let handle_value = proc.add_handle(handle);

        // getting object should success
        let object: Arc<Process> = proc
            .get_object_with_rights(handle_value, Rights::DEFAULT_PROCESS)
            .expect("failed to get object");
        assert!(Arc::ptr_eq(&object, &proc));

        let (object, rights) = proc
            .get_object_and_rights::<Process>(handle_value)
            .expect("failed to get object");
        assert!(Arc::ptr_eq(&object, &proc));
        assert_eq!(rights, Rights::DEFAULT_PROCESS);

        // getting object with an extra rights should fail.
        assert_eq!(
            proc.get_object_with_rights::<Process>(handle_value, Rights::MANAGE_JOB)
                .err(),
            Some(ZxError::ACCESS_DENIED)
        );

        // getting object with invalid type should fail.
        assert_eq!(
            proc.get_object_with_rights::<Job>(handle_value, Rights::DEFAULT_PROCESS)
                .err(),
            Some(ZxError::WRONG_TYPE)
        );

        proc.remove_handle(handle_value).unwrap();

        // getting object with invalid handle should fail.
        assert_eq!(
            proc.get_object_with_rights::<Process>(handle_value, Rights::DEFAULT_PROCESS)
                .err(),
            Some(ZxError::BAD_HANDLE)
        );

        let handle1 = Handle::new(proc.clone(), Rights::DEFAULT_PROCESS);
        let handle2 = Handle::new(proc.clone(), Rights::DEFAULT_PROCESS);

        let handle_values = proc.add_handles(vec![handle1, handle2]);
        let object1: Arc<Process> = proc
            .get_object_with_rights(handle_values[0], Rights::DEFAULT_PROCESS)
            .expect("failed to get object");
        assert!(Arc::ptr_eq(&object1, &proc));

        proc.remove_handles(&handle_values).unwrap();
        assert_eq!(
            proc.get_object_with_rights::<Process>(handle_values[0], Rights::DEFAULT_PROCESS)
                .err(),
            Some(ZxError::BAD_HANDLE)
        );
    }

    #[test]
    fn get_child() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        assert_eq!(proc.get_child(thread.id()).unwrap().id(), thread.id());
        assert_eq!(proc.get_child(proc.id()).err(), Some(ZxError::NOT_FOUND));

        let thread1 = Thread::create(&proc, "thread1").expect("failed to create thread");
        assert_eq!(proc.thread_ids(), vec![thread.id(), thread1.id()]);
    }

    #[test]
    fn contains_thread() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        let proc1 = Process::create(&root_job, "proc1").expect("failed to create process");
        let thread1 = Thread::create(&proc1, "thread1").expect("failed to create thread");

        let inner = proc.inner.lock();
        assert!(inner.contains_thread(&thread) && !inner.contains_thread(&thread1));
    }

    #[test]
    fn check_policy() {
        let root_job = Job::root();
        let policy1 = BasicPolicy {
            condition: PolicyCondition::BadHandle,
            action: PolicyAction::Allow,
        };
        let policy2 = BasicPolicy {
            condition: PolicyCondition::NewChannel,
            action: PolicyAction::Deny,
        };

        assert!(root_job
            .set_policy_basic(SetPolicyOptions::Absolute, &[policy1, policy2])
            .is_ok());
        let proc = Process::create(&root_job, "proc").expect("failed to create process");

        assert!(proc.check_policy(PolicyCondition::BadHandle).is_ok());
        assert!(proc.check_policy(PolicyCondition::NewProcess).is_ok());
        assert_eq!(
            proc.check_policy(PolicyCondition::NewChannel).err(),
            Some(ZxError::ACCESS_DENIED)
        );

        let _job = root_job.create_child().unwrap();
        assert_eq!(
            root_job
                .set_policy_basic(SetPolicyOptions::Absolute, &[policy1, policy2])
                .err(),
            Some(ZxError::BAD_STATE)
        );
    }

    #[test]
    fn exit() {
        let root_job = Job::root();
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        let info = proc.get_info();
        assert!(!info.has_exited && !info.started && info.return_code == 0);

        proc.exit(666);
        let info = proc.get_info();
        assert!(info.has_exited && info.started && info.return_code == 666);
        assert_eq!(thread.state(), ThreadState::Dying);
        // TODO: when is the thread dead?

        assert_eq!(
            Thread::create(&proc, "thread1").err(),
            Some(ZxError::BAD_STATE)
        );
    }
}
