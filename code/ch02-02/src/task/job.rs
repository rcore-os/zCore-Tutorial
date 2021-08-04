use {
    super::job_policy::*,
    super::process::Process,
    super::*,
    crate::error::*,
    crate::object::*,
    crate::task::Task,
    alloc::sync::{Arc, Weak},
    alloc::vec::Vec,
    spin::Mutex,
};

/// Job 对象
#[allow(dead_code)]
pub struct Job {
    base: KObjectBase,
    parent: Option<Arc<Job>>,
    parent_policy: JobPolicy,
    inner: Mutex<JobInner>,
}

impl_kobject!(Job
    fn get_child(&self, id: KoID) -> ZxResult<Arc<dyn KernelObject>> {
        let inner = self.inner.lock();
        if let Some(job) = inner.children.iter().filter_map(|o|o.upgrade()).find(|o| o.id() == id) {
            return Ok(job);
        }
        if let Some(proc) = inner.processes.iter().find(|o| o.id() == id) {
            return Ok(proc.clone());
        }
        Err(ZxError::NOT_FOUND)
    }
    fn related_koid(&self) -> KoID {
        self.parent.as_ref().map(|p| p.id()).unwrap_or(0)
    }
);

#[derive(Default)]
struct JobInner {
    policy: JobPolicy,
    children: Vec<Weak<Job>>,
    processes: Vec<Arc<Process>>,
    // if the job is killed, no more child creation should works
    killed: bool,
    self_ref: Weak<Job>,
}

impl Job {
    /// Create the root job.
    pub fn root() -> Arc<Self> {
        let job = Arc::new(Job {
            base: KObjectBase::new(),
            parent: None,
            parent_policy: JobPolicy::default(),
            inner: Mutex::new(JobInner::default()),
        });
        job.inner.lock().self_ref = Arc::downgrade(&job);
        job
    }

    /// Create a new child job object.
    pub fn create_child(self: &Arc<Self>) -> ZxResult<Arc<Self>> {
        let mut inner = self.inner.lock();
        if inner.killed {
            return Err(ZxError::BAD_STATE);
        }
        let child = Arc::new(Job {
            base: KObjectBase::new(),
            parent: Some(self.clone()),
            parent_policy: inner.policy.merge(&self.parent_policy),
            inner: Mutex::new(JobInner::default()),
        });
        let child_weak = Arc::downgrade(&child);
        child.inner.lock().self_ref = child_weak.clone();
        inner.children.push(child_weak);
        Ok(child)
    }

    fn remove_child(&self, to_remove: &Weak<Job>) {
        let mut inner = self.inner.lock();
        inner.children.retain(|child| !to_remove.ptr_eq(child));
        if inner.killed && inner.processes.is_empty() && inner.children.is_empty() {
            drop(inner);
            self.terminate()
        }
    }

    /// Get the policy of the job.
    pub fn policy(&self) -> JobPolicy {
        self.inner.lock().policy.merge(&self.parent_policy)
    }

    /// Get the parent job.
    pub fn parent(&self) -> Option<Arc<Self>> {
        self.parent.clone()
    }

    /// Sets one or more security and/or resource policies to an empty job.
    ///
    /// The job's effective policies is the combination of the parent's
    /// effective policies and the policies specified in policy.
    ///
    /// After this call succeeds any new child process or child job will have
    /// the new effective policy applied to it.
    pub fn set_policy_basic(
        &self,
        options: SetPolicyOptions,
        policies: &[BasicPolicy],
    ) -> ZxResult {
        let mut inner = self.inner.lock();
        if !inner.is_empty() {
            return Err(ZxError::BAD_STATE);
        }
        for policy in policies {
            if self.parent_policy.get_action(policy.condition).is_some() {
                match options {
                    SetPolicyOptions::Absolute => return Err(ZxError::ALREADY_EXISTS),
                    SetPolicyOptions::Relative => {}
                }
            } else {
                inner.policy.apply(*policy);
            }
        }
        Ok(())
    }

    /// Add a process to the job.
    pub(super) fn add_process(&self, process: Arc<Process>) -> ZxResult {
        let mut inner = self.inner.lock();
        if inner.killed {
            return Err(ZxError::BAD_STATE);
        }
        inner.processes.push(process);
        Ok(())
    }

    /// Remove a process from the job.
    pub(super) fn remove_process(&self, id: KoID) {
        let mut inner = self.inner.lock();
        inner.processes.retain(|proc| proc.id() != id);
        if inner.killed && inner.processes.is_empty() && inner.children.is_empty() {
            drop(inner);
            self.terminate()
        }
    }

    /// Check whether this job is root job.
    pub fn check_root_job(&self) -> ZxResult {
        if self.parent.is_some() {
            Err(ZxError::ACCESS_DENIED)
        } else {
            Ok(())
        }
    }

    /// Get KoIDs of Processes.
    pub fn process_ids(&self) -> Vec<KoID> {
        self.inner.lock().processes.iter().map(|p| p.id()).collect()
    }

    /// Get KoIDs of children Jobs.
    pub fn children_ids(&self) -> Vec<KoID> {
        self.inner
            .lock()
            .children
            .iter()
            .filter_map(|j| j.upgrade())
            .map(|j| j.id())
            .collect()
    }

    /// Return true if this job has no processes and no child jobs.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    /// The job finally terminates.
    fn terminate(&self) {
        if let Some(parent) = self.parent.as_ref() {
            parent.remove_child(&self.inner.lock().self_ref)
        }
    }
}

impl Task for Job {
    /// Kill the job. The job do not terminate immediately when killed.
    /// It will terminate after all its children and processes are terminated.
    fn kill(&self) {
        let (children, processes) = {
            let mut inner = self.inner.lock();
            if inner.killed {
                return;
            }
            inner.killed = true;
            (inner.children.clone(), inner.processes.clone())
        };
        if children.is_empty() && processes.is_empty() {
            self.terminate();
            return;
        }
        for child in children {
            if let Some(child) = child.upgrade() {
                child.kill();
            }
        }
        for proc in processes {
            proc.kill();
        }
    }

    fn suspend(&self) {
        panic!("job do not support suspend");
    }

    fn resume(&self) {
        panic!("job do not support resume");
    }
}

impl JobInner {
    fn is_empty(&self) -> bool {
        self.processes.is_empty() && self.children.is_empty()
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TASK_RETCODE_SYSCALL_KILL;

    #[test]
    fn create() {
        let root_job = Job::root();
        let job = Job::create_child(&root_job).expect("failed to create job");

        let child = root_job
            .get_child(job.id())
            .unwrap()
            .downcast_arc()
            .unwrap();
        assert!(Arc::ptr_eq(&child, &job));
        assert_eq!(job.related_koid(), root_job.id());
        assert_eq!(root_job.related_koid(), 0);

        root_job.kill();
        assert_eq!(root_job.create_child().err(), Some(ZxError::BAD_STATE));
    }

    #[test]
    fn set_policy() {
        let root_job = Job::root();

        // default policy
        assert_eq!(
            root_job.policy().get_action(PolicyCondition::BadHandle),
            None
        );

        // set policy for root job
        let policy = &[BasicPolicy {
            condition: PolicyCondition::BadHandle,
            action: PolicyAction::Deny,
        }];
        root_job
            .set_policy_basic(SetPolicyOptions::Relative, policy)
            .expect("failed to set policy");
        assert_eq!(
            root_job.policy().get_action(PolicyCondition::BadHandle),
            Some(PolicyAction::Deny)
        );

        // override policy should success
        let policy = &[BasicPolicy {
            condition: PolicyCondition::BadHandle,
            action: PolicyAction::Allow,
        }];
        root_job
            .set_policy_basic(SetPolicyOptions::Relative, policy)
            .expect("failed to set policy");
        assert_eq!(
            root_job.policy().get_action(PolicyCondition::BadHandle),
            Some(PolicyAction::Allow)
        );

        // create a child job
        let job = Job::create_child(&root_job).expect("failed to create job");

        // should inherit parent's policy.
        assert_eq!(
            job.policy().get_action(PolicyCondition::BadHandle),
            Some(PolicyAction::Allow)
        );

        // setting policy for a non-empty job should fail.
        assert_eq!(
            root_job.set_policy_basic(SetPolicyOptions::Relative, &[]),
            Err(ZxError::BAD_STATE)
        );

        // set new policy should success.
        let policy = &[BasicPolicy {
            condition: PolicyCondition::WrongObject,
            action: PolicyAction::Allow,
        }];
        job.set_policy_basic(SetPolicyOptions::Relative, policy)
            .expect("failed to set policy");
        assert_eq!(
            job.policy().get_action(PolicyCondition::WrongObject),
            Some(PolicyAction::Allow)
        );

        // relatively setting existing policy should be ignored.
        let policy = &[BasicPolicy {
            condition: PolicyCondition::BadHandle,
            action: PolicyAction::Deny,
        }];
        job.set_policy_basic(SetPolicyOptions::Relative, policy)
            .expect("failed to set policy");
        assert_eq!(
            job.policy().get_action(PolicyCondition::BadHandle),
            Some(PolicyAction::Allow)
        );

        // absolutely setting existing policy should fail.
        assert_eq!(
            job.set_policy_basic(SetPolicyOptions::Absolute, policy),
            Err(ZxError::ALREADY_EXISTS)
        );
    }

    #[test]
    fn parent_child() {
        let root_job = Job::root();
        let job = Job::create_child(&root_job).expect("failed to create job");
        let proc = Process::create(&root_job, "proc").expect("failed to create process");

        assert_eq!(root_job.get_child(job.id()).unwrap().id(), job.id());
        assert_eq!(root_job.get_child(proc.id()).unwrap().id(), proc.id());
        assert_eq!(
            root_job.get_child(root_job.id()).err(),
            Some(ZxError::NOT_FOUND)
        );
        assert!(Arc::ptr_eq(&job.parent().unwrap(), &root_job));

        let job1 = root_job.create_child().expect("failed to create job");
        let proc1 = Process::create(&root_job, "proc1").expect("failed to create process");
        assert_eq!(root_job.children_ids(), vec![job.id(), job1.id()]);
        assert_eq!(root_job.process_ids(), vec![proc.id(), proc1.id()]);

        root_job.kill();
        assert_eq!(root_job.create_child().err(), Some(ZxError::BAD_STATE));
    }

    #[test]
    fn check() {
        let root_job = Job::root();
        assert!(root_job.is_empty());
        let job = root_job.create_child().expect("failed to create job");
        assert_eq!(root_job.check_root_job(), Ok(()));
        assert_eq!(job.check_root_job(), Err(ZxError::ACCESS_DENIED));

        assert!(!root_job.is_empty());
        assert!(job.is_empty());

        let _proc = Process::create(&job, "proc").expect("failed to create process");
        assert!(!job.is_empty());
    }

    #[test]
    fn kill() {
        let root_job = Job::root();
        let job = Job::create_child(&root_job).expect("failed to create job");
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        let thread = Thread::create(&proc, "thread").expect("failed to create thread");

        root_job.kill();
        assert!(root_job.inner.lock().killed);
        assert!(job.inner.lock().killed);
        assert_eq!(proc.status(), Status::Exited(TASK_RETCODE_SYSCALL_KILL));
        // assert_eq!(thread.state(), ThreadState::Dying);

        std::mem::drop(thread);
        assert!(root_job.inner.lock().killed);
        assert!(job.inner.lock().killed);
        assert_eq!(proc.status(), Status::Exited(TASK_RETCODE_SYSCALL_KILL));
        // assert_eq!(thread.state(), ThreadState::Dead);

        // The job has no children.
        let root_job = Job::root();
        root_job.kill();
        assert!(root_job.inner.lock().killed);

        // The job's process have no threads.
        let root_job = Job::root();
        let job = Job::create_child(&root_job).expect("failed to create job");
        let proc = Process::create(&root_job, "proc").expect("failed to create process");
        root_job.kill();
        assert!(root_job.inner.lock().killed);
        assert!(job.inner.lock().killed);
        assert_eq!(proc.status(), Status::Exited(TASK_RETCODE_SYSCALL_KILL));
    }
}
