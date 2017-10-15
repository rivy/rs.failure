extern crate backtrace;

use std::cell::UnsafeCell;
use std::env;
use std::fmt;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::sync::Mutex;

pub use self::backtrace::Backtrace;

/// Internal representation of a backtrace
#[doc(hidden)]
pub(crate) struct InternalBacktrace {
    backtrace: Option<MaybeResolved>,
}

struct MaybeResolved {
    resolved: Mutex<bool>,
    backtrace: UnsafeCell<Backtrace>,
}

unsafe impl Send for MaybeResolved {}
unsafe impl Sync for MaybeResolved {}

impl InternalBacktrace {
    /// Returns a backtrace of the current call stack if `RUST_BACKTRACE`
    /// is set to anything but ``0``, and `None` otherwise.  This is used
    /// in the generated error implementations.
    #[doc(hidden)]
    pub fn new() -> InternalBacktrace {
        static ENABLED: AtomicUsize = ATOMIC_USIZE_INIT;

        match ENABLED.load(Ordering::SeqCst) {
            0 => {
                let enabled = match env::var_os("RUST_BACKTRACE") {
                    Some(ref val) if val != "0" => true,
                    _ => false,
                };
                ENABLED.store(enabled as usize + 1, Ordering::SeqCst);
                if !enabled {
                    return InternalBacktrace { backtrace: None }
                }
            }
            1 => return InternalBacktrace { backtrace: None },
            _ => {}
        }

        InternalBacktrace {
            backtrace: Some(MaybeResolved {
                resolved: Mutex::new(false),
                backtrace: UnsafeCell::new(Backtrace::new_unresolved()),
            }),
        }
    }

    pub fn none() -> InternalBacktrace {
        InternalBacktrace { backtrace: None }
    }

    /// Acquire the internal backtrace
    #[doc(hidden)]
    pub fn as_backtrace(&self) -> Option<&Backtrace> {
        let bt = match self.backtrace {
            Some(ref bt) => bt,
            None => return None,
        };
        let mut resolved = bt.resolved.lock().unwrap();
        unsafe {
            if !*resolved {
                (*bt.backtrace.get()).resolve();
                *resolved = true;
            }
            Some(&*bt.backtrace.get())
        }
    }
}

impl fmt::Debug for InternalBacktrace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InternalBacktrace")
            .field("backtrace", &self.as_backtrace())
            .finish()
    }
}
