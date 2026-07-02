//! OS-specific process supervision (docs/architecture.md section 4).
//!
//! On Windows, every managed child is assigned to a Job Object configured
//! with KILL_ON_JOB_CLOSE. The job handle lives for the lifetime of the
//! Omnira process; when the process dies for any reason (including crash or
//! force-kill), Windows closes the handle and terminates every process in
//! the job. This is the guarantee against orphaned llama-server.exe.
//!
//! This module is the isolation seam for future macOS/Linux support.

use std::sync::OnceLock;

#[cfg(windows)]
mod win {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    /// A Job Object handle that kills all member processes when dropped
    /// (or when this process dies).
    pub struct Job(HANDLE);

    // HANDLE is a raw pointer wrapper; the job handle is only used behind
    // a OnceLock and the underlying object is thread-safe kernel state.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn new() -> std::io::Result<Self> {
            unsafe {
                let job = CreateJobObjectW(None, None)
                    .map_err(|e| std::io::Error::other(format!("CreateJobObject: {e}")))?;
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .map_err(|e| std::io::Error::other(format!("SetInformationJobObject: {e}")))?;
                Ok(Self(job))
            }
        }

        /// Assign a child process (by PID) to this job.
        pub fn assign_pid(&self, pid: u32) -> std::io::Result<()> {
            unsafe {
                let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
                    .map_err(|e| std::io::Error::other(format!("OpenProcess({pid}): {e}")))?;
                let result = AssignProcessToJobObject(self.0, process)
                    .map_err(|e| std::io::Error::other(format!("AssignProcessToJobObject: {e}")));
                let _ = windows::Win32::Foundation::CloseHandle(process);
                result
            }
        }
    }
}

#[cfg(windows)]
static JOB: OnceLock<win::Job> = OnceLock::new();

/// Assign a spawned child process to the app-wide kill-on-close Job Object.
/// Must be called immediately after spawn, before any await point that could
/// let the child outlive a crash of this process unsupervised.
pub fn supervise(pid: u32) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let job = match JOB.get() {
            Some(j) => j,
            None => {
                let created = win::Job::new()?;
                JOB.get_or_init(|| created)
            }
        };
        job.assign_pid(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        Ok(())
    }
}
