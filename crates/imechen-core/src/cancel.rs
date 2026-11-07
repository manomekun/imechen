use ffmpeg_sidecar::child::FfmpegChild;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

type SharedChild = Arc<Mutex<FfmpegChild>>;

struct Inner {
    cancelled: AtomicBool,
    child: Mutex<Option<SharedChild>>,
}

static REGISTRY: Mutex<Vec<Weak<Inner>>> = Mutex::new(Vec::new());
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// A poisoned lock only means another thread panicked mid-cancel; the state
/// is still valid and cancelling must keep working during shutdown.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Shared flag a worker polls to stop early; clones observe the same state.
/// Cancelling also kills the ffmpeg process attached to it, so a stop does not
/// have to wait for the next ffmpeg event.
#[derive(Clone)]
pub struct CancelToken(Arc<Inner>);

impl Default for CancelToken {
    fn default() -> Self {
        let inner = Arc::new(Inner {
            cancelled: AtomicBool::new(false),
            child: Mutex::new(None),
        });
        let mut registry = lock(&REGISTRY);
        registry.retain(|w| w.strong_count() > 0);
        registry.push(Arc::downgrade(&inner));
        Self(inner)
    }
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        // The flag is set before the slot lock is taken and attach() checks it
        // after taking the slot lock, so one side always sees the other.
        self.0.cancelled.store(true, Ordering::SeqCst);
        // Killing under the slot lock is safe: detach() takes the same lock
        // before wait() locks the child, so the child is never locked here.
        if let Some(child) = lock(&self.0.child).as_ref() {
            kill(&mut lock(child));
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) fn attach(&self, child: SharedChild) {
        let mut slot = lock(&self.0.child);
        *slot = Some(child.clone());
        if self.is_cancelled() {
            kill(&mut lock(&child));
        }
    }

    /// Must run before blocking on the child's `wait()`: that holds the child
    /// lock until ffmpeg exits, and a concurrent `cancel()` would block on it.
    pub(crate) fn detach(&self) {
        *lock(&self.0.child) = None;
    }
}

/// A package-manager shim (Chocolatey, Scoop) runs the real ffmpeg as its own
/// child, and killing only the shim leaves that child encoding, so Windows
/// kills the whole process tree.
fn kill(child: &mut FfmpegChild) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // A GUI app would otherwise flash a console window for taskkill.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let pid = child.as_inner().id().to_string();
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid, "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = child.kill();
}

/// Cancels every live token, for app shutdown where the page that owns a token
/// may already be gone.
pub fn cancel_all() {
    cancel_registered(&REGISTRY);
}

fn cancel_registered(registry: &Mutex<Vec<Weak<Inner>>>) {
    let tokens: Vec<Arc<Inner>> = lock(registry).iter().filter_map(Weak::upgrade).collect();
    for inner in tokens {
        CancelToken(inner).cancel();
    }
}

/// Counts conversions that have not finished cleaning up.
pub fn active_jobs() -> usize {
    ACTIVE.load(Ordering::SeqCst)
}

/// Declare it as the first local of a job: locals drop in reverse order, so it
/// drops last, after `OutputReservation` has removed any partial file.
pub(crate) struct JobGuard;

impl JobGuard {
    pub(crate) fn new() -> Self {
        ACTIVE.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for JobGuard {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Cancellation is not an error: carrying it in `Err` would surface a
/// user-requested stop as a failed conversion.
#[derive(Debug, PartialEq)]
pub enum Outcome<T> {
    Done(T),
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_a_clone_is_observed_by_every_clone() {
        let token = CancelToken::new();
        let other = token.clone();
        assert!(!other.is_cancelled());

        token.cancel();

        assert!(other.is_cancelled());
    }

    // Exercises a private registry: cancelling the global one would also
    // cancel conversions running in parallel tests.
    #[test]
    fn cancelling_a_registry_cancels_its_live_tokens_only() {
        let registered = CancelToken::new();
        let other = CancelToken::new();
        let dropped = CancelToken::new();
        let registry = Mutex::new(vec![
            Arc::downgrade(&registered.0),
            Arc::downgrade(&dropped.0),
        ]);
        drop(dropped);

        cancel_registered(&registry);

        assert!(registered.is_cancelled());
        assert!(!other.is_cancelled());
    }

    #[test]
    fn new_tokens_are_registered_for_cancel_all() {
        let token = CancelToken::new();

        assert!(lock(&REGISTRY)
            .iter()
            .any(|w| w.upgrade().is_some_and(|i| Arc::ptr_eq(&i, &token.0))));
    }

    #[test]
    fn job_guard_counts_only_while_alive() {
        let guard = JobGuard::new();
        assert!(active_jobs() >= 1);
        drop(guard);
    }
}
