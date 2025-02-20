use libc::{c_char, pid_t};
use std::{
    ffi::CString,
    io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult},
};

unsafe extern "C" {
    static environ: *const *const c_char;
}

pub enum ForkReturn {
    Parent(pid_t),
    Child,
}

pub(crate) fn fork() -> ForkReturn {
    let res = unsafe { libc::fork() };

    // TODO: Use `assert!` (or `assert_eq!`) here to make sure we only have one thread

    if res < 0 {
        panic!("fork failed")
    } else {
        if res == 0 {
            ForkReturn::Child
        } else {
            ForkReturn::Parent(res)
        }
    }
}

pub(crate) fn exec<S: AsRef<str>>(pathname: &S, argv: &[S]) -> IOResult<()> {
    let pathname = CString::new(pathname.as_ref()).map_err(|_| {
        IOError::new(
            IOErrorKind::InvalidInput,
            "BAD: pathname str had a null byte.",
        )
    })?;

    // Store our CStrings
    let argv = argv
        .iter()
        .map(|arg| CString::new(arg.as_ref()))
        .filter_map(|res| res.ok())
        .collect::<Vec<_>>();

    let mut argv_ptrs = argv.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();
    argv_ptrs.push(std::ptr::null());

    if unsafe { libc::execvpe(pathname.as_ptr(), argv_ptrs.as_ptr(), environ) } < 0 {
        Err(IOError::last_os_error())
    } else {
        unsafe {
            std::hint::unreachable_unchecked();
        }
    }
}

pub(crate) struct WaitReturn {
    pid: pid_t,
    status: WaitStatus,
}

pub(crate) enum WaitStatus {
    Exited(i32),
    TermSignal(i32),
    Stopped(i32),
    Continued,
    Unknown,
}

impl From<WaitReturn> for WaitStatus {
    fn from(value: WaitReturn) -> Self {
        value.status
    }
}

pub(crate) fn wait() -> IOResult<WaitReturn> {
    use WaitStatus as WS;
    use libc::{WIFEXITED, WEXITSTATUS, WIFSIGNALED, WTERMSIG, WIFSTOPPED, WSTOPSIG, WIFCONTINUED};

    let mut stat_code = 0i32;

    let res = unsafe { libc::wait(&raw mut stat_code) };

    if res < 0 {
        Err(IOError::last_os_error())
    } else {
        let pid = res;

        let status = if WIFEXITED(stat_code) {
            WS::Exited(WEXITSTATUS(stat_code))
        } else if WIFSIGNALED(stat_code) {
            WS::TermSignal(WTERMSIG(stat_code))
        } else if WIFSTOPPED(stat_code) {
            WS::Stopped(WSTOPSIG(stat_code))
        } else if WIFCONTINUED(stat_code) {
            WS::Continued
        } else {
            WS::Unknown
        };

        Ok(WaitReturn { pid, status })
    }
}
