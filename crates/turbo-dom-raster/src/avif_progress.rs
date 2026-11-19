//! Test-only last-mutation storage; recording must not change decoder timing with I/O.
use std::io;
use std::process::Command;

#[cfg(unix)]
const ENV: &str = "TURBO_DOM_AVIF_FUZZ_PROGRESS";

pub(super) fn case_id(fixture: usize, plane: usize, variant: usize) -> u64 {
    (u64::from(u16::try_from(fixture).unwrap()) << 48)
        | (u64::from(u16::try_from(plane).unwrap()) << 32)
        | u64::from(u32::try_from(variant).unwrap().checked_add(1).unwrap())
}

pub(super) struct Breadcrumb {
    #[cfg(unix)]
    pointer: std::ptr::NonNull<std::sync::atomic::AtomicU64>,
    #[cfg(unix)]
    path: std::path::PathBuf,
    #[cfg(unix)]
    owner: bool,
}

#[cfg(unix)]
impl Breadcrumb {
    pub(super) fn new() -> io::Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "turbo-dom-avif-{}-{nonce}.progress",
            std::process::id()
        ));
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.set_len(std::mem::size_of::<std::sync::atomic::AtomicU64>() as u64)?;
        Self::map(&file, path, true)
    }

    pub(super) fn child() -> io::Result<Self> {
        match std::env::var_os(ENV) {
            Some(path) => Self::open(path.into()),
            None => Self::new(),
        }
    }

    fn open(path: std::path::PathBuf) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;
        Self::map(&file, path, false)
    }

    fn map(file: &std::fs::File, path: std::path::PathBuf, owner: bool) -> io::Result<Self> {
        use std::os::fd::AsRawFd;
        if file.metadata()?.len() < std::mem::size_of::<std::sync::atomic::AtomicU64>() as u64 {
            return Err(io::Error::other("short AVIF progress file"));
        }
        // SAFETY: the file owns one zero-initialized atomic-sized slot. mmap
        // returns page-aligned storage, retained until this mapping is dropped.
        let memory = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                std::mem::size_of::<std::sync::atomic::AtomicU64>(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if memory == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let Some(pointer) = std::ptr::NonNull::new(memory.cast()) else {
            // SAFETY: mmap succeeded; reject address zero rather than forming a reference.
            unsafe { libc::munmap(memory, std::mem::size_of::<std::sync::atomic::AtomicU64>()) };
            return Err(io::Error::other("null AVIF progress mapping"));
        };
        Ok(Self {
            pointer,
            path,
            owner,
        })
    }

    pub(super) fn configure(&self, command: &mut Command) {
        command.env(ENV, &self.path);
    }

    pub(super) fn record(&self, fixture: usize, plane: usize, variant: usize) {
        // SAFETY: this mapping contains one aligned AtomicU64. The child is its
        // sole writer; the parent reads only after that process has exited.
        unsafe { self.pointer.as_ref() }.store(
            case_id(fixture, plane, variant),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    pub(super) fn read(&self, _stderr: &str) -> u64 {
        // SAFETY: the mapping is still owned by self and the child has exited.
        unsafe { self.pointer.as_ref() }.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(unix)]
impl Drop for Breadcrumb {
    fn drop(&mut self) {
        // SAFETY: this is exactly the live mapping created by mmap above.
        unsafe {
            libc::munmap(
                self.pointer.as_ptr().cast(),
                std::mem::size_of::<std::sync::atomic::AtomicU64>(),
            )
        };
        if self.owner {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(not(unix))]
impl Breadcrumb {
    pub(super) fn new() -> io::Result<Self> {
        Ok(Self {})
    }
    pub(super) fn child() -> io::Result<Self> {
        Self::new()
    }
    pub(super) fn configure(&self, _command: &mut Command) {}
    pub(super) fn record(&self, fixture: usize, plane: usize, variant: usize) {
        eprintln!("FUZZ-ID {}", case_id(fixture, plane, variant));
    }
    pub(super) fn read(&self, stderr: &str) -> u64 {
        stderr
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("FUZZ-ID ")?.parse().ok())
            .unwrap_or(0)
    }
}

#[cfg(unix)]
#[test]
fn independently_mapped_child_progress_survives_until_parent_reads_it() {
    let parent = Breadcrumb::new().unwrap();
    let path = parent.path.clone();
    assert_eq!(parent.read(""), 0);
    let child = Breadcrumb::open(path.clone()).unwrap();
    child.record(17, 2, 65537);
    drop(child);
    assert_eq!(parent.read(""), case_id(17, 2, 65537));
    drop(parent);
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn a_nonzero_child_exit_retains_its_last_atomic_case_without_stderr() {
    const PROBE: &str = "TURBO_DOM_AVIF_PROGRESS_PROBE";
    if std::env::var_os(PROBE).is_some() {
        Breadcrumb::child().unwrap().record(7, 1, 1234);
        std::process::exit(23);
    }
    let parent = Breadcrumb::new().unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    parent.configure(&mut command);
    let output = command
        .env(PROBE, "1")
        .args([
            "--exact",
            "avif::progress::a_nonzero_child_exit_retains_its_last_atomic_case_without_stderr",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(23));
    assert!(output.stderr.is_empty());
    assert_eq!(parent.read(""), case_id(7, 1, 1234));
}

#[cfg(unix)]
#[test]
fn truncated_progress_storage_is_rejected_before_mapping() {
    let parent = Breadcrumb::new().unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&parent.path)
        .unwrap()
        .set_len(0)
        .unwrap();
    assert!(Breadcrumb::open(parent.path.clone()).is_err());
}
