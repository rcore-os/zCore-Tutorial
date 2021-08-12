use {super::process::Process, super::*, crate::object::*, alloc::sync::Arc};

pub struct Thread {
    base: KObjectBase,
    proc: Arc<Process>,
}

impl_kobject!(Thread
    fn related_koid(&self) -> KoID {
        self.proc.id()
    }
);

impl Thread {
    /// Create a new thread.
    pub fn create(proc: &Arc<Process>, name: &str) -> ZxResult<Arc<Self>> {
        let thread = Arc::new(Thread {
            base: KObjectBase::with_name(name),
            proc: proc.clone(),
        });
        proc.add_thread(thread.clone())?;
        Ok(thread)
    }
}
