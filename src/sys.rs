//! Safe wrappers around the libc calls this tool needs. All `unsafe` code lives here.

use std::ffi::CStr;
use std::io;
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::ptr;

/// Returns this machine's host name, or an empty string if it cannot be read.
pub fn hostname() -> String {
    let mut buf = [0u8; 256];
    // SAFETY: `buf` is valid for writes of `buf.len()` bytes.
    if unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) } != 0 {
        return String::new();
    }
    CStr::from_bytes_until_nul(&buf).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Opens a pseudo-terminal and returns `(master, slave)`.
///
/// The pty gets the window size of our stdout (if that is a terminal) and has output
/// post-processing turned off. Both ends are close-on-exec, so a child only gets the slave
/// where it is explicitly given one (e.g. as its stdout).
pub fn open_pty() -> io::Result<(OwnedFd, OwnedFd)> {
    let size = window_size(&io::stdout());
    let size_ptr = size.as_ref().map_or(ptr::null(), ptr::from_ref);
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: `master` and `slave` are valid out-pointers; name and termios may be null;
    // `size_ptr` is null or points to `size`, which outlives the call.
    if unsafe { libc::openpty(&raw mut master, &raw mut slave, ptr::null_mut(), ptr::null(), size_ptr) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openpty succeeded, so both are open file descriptors that nothing else owns.
    let (master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };

    set_cloexec(&master)?;
    set_cloexec(&slave)?;
    disable_output_processing(&slave);
    Ok((master, slave))
}

/// Ignores SIGINT and SIGQUIT in this process.
///
/// Ctrl+C still reaches the child (it is in the same process group), and we keep running so we
/// can copy the rest of its output.
pub fn ignore_interrupts() {
    // SAFETY: SIG_IGN is a valid disposition, so no handler code runs.
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
    }
}

/// Returns the window size of the terminal on `fd`, or `None` if it is not a terminal.
fn window_size(fd: &impl AsFd) -> Option<libc::winsize> {
    let mut size = MaybeUninit::<libc::winsize>::uninit();
    // SAFETY: TIOCGWINSZ writes a `winsize` through the pointer.
    if unsafe { libc::ioctl(fd.as_fd().as_raw_fd(), libc::TIOCGWINSZ, size.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: the ioctl succeeded, so `size` is initialised.
    Some(unsafe { size.assume_init() })
}

/// Sets `FD_CLOEXEC` on `fd`, so it is closed in child processes.
fn set_cloexec(fd: &OwnedFd) -> io::Result<()> {
    // SAFETY: `fd` is an open file descriptor; F_SETFD takes an int argument.
    if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Turns off output post-processing (OPOST) on a terminal: the real terminal we copy to already
/// turns `\n` into `\r\n`. Best effort: errors are ignored.
fn disable_output_processing(fd: &OwnedFd) {
    let mut tio = MaybeUninit::<libc::termios>::uninit();
    // SAFETY: tcgetattr writes a `termios` through the pointer.
    if unsafe { libc::tcgetattr(fd.as_raw_fd(), tio.as_mut_ptr()) } != 0 {
        return;
    }
    // SAFETY: tcgetattr succeeded, so `tio` is initialised.
    let mut tio = unsafe { tio.assume_init() };
    tio.c_oflag &= !libc::OPOST;
    // SAFETY: `tio` is a valid `termios` for the duration of the call.
    unsafe { libc::tcsetattr(fd.as_raw_fd(), libc::TCSANOW, &raw const tio) };
}
