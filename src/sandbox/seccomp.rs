//! Seccomp system call filter backend.

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("the seccomp feature supports only x86_64 and aarch64");

use libseccomp::{ScmpAction, ScmpArgCompare, ScmpCompareOp, ScmpFilterContext, ScmpSyscall, error::SeccompError};

use crate::warning;

// System calls that ouch makes on both architectures after the sandbox starts, found with strace
const ALLOWED: &[&str] = &[
    "read",
    "write",
    "lseek",
    "close",
    "fcntl",
    "openat",
    "fstat",
    "newfstatat",
    "statx",
    "getdents64",
    "getcwd",
    "chdir",
    "unlinkat",
    "linkat",
    "fchmod",
    "utimensat",
    "umask",
    "brk",
    "mmap",
    "munmap",
    "mremap",
    "mprotect",
    "madvise",
    "futex",
    "set_robust_list",
    "rseq",
    "sched_getaffinity",
    "sched_yield",
    "getpid",
    "gettid",
    "exit",
    "exit_group",
    "rt_sigprocmask",
    "sigaltstack",
    "getrandom",
    // Crash handling and resuming after SIGSTOP
    "rt_sigaction",
    "rt_sigreturn",
    "restart_syscall",
    // Used when the vDSO cannot read the clock
    "clock_gettime",
];

// glibc and musl use these path calls on x86_64
#[cfg(target_arch = "x86_64")]
const ALLOWED_ARCH: &[&str] = &[
    "open", "stat", "lstat", "access", "readlink", "mkdir", "rmdir", "unlink", "rename", "symlink", "link", "chmod",
];

// aarch64 has only the *at variants of the path calls
#[cfg(target_arch = "aarch64")]
const ALLOWED_ARCH: &[&str] = &[
    "faccessat",
    "readlinkat",
    "mkdirat",
    "symlinkat",
    "fchmodat",
    "renameat",
];

// Newer calls that libc tries first get ENOSYS, which makes it fall back to older calls
const FALLBACK: &[&str] = &[
    "clone3",
    "close_range",
    "openat2",
    "mseal",
    "faccessat2",
    "fchmodat2",
    "map_shadow_stack",
    "futex_wake",
    "futex_wait",
    "futex_requeue",
];

// glibc starts threads with these flags and musl adds CLONE_DETACHED
const THREAD_FLAGS: libc::c_int = libc::CLONE_VM
    | libc::CLONE_FS
    | libc::CLONE_FILES
    | libc::CLONE_SIGHAND
    | libc::CLONE_THREAD
    | libc::CLONE_SYSVSEM
    | libc::CLONE_SETTLS
    | libc::CLONE_PARENT_SETTID
    | libc::CLONE_CHILD_CLEARTID
    | if cfg!(target_env = "musl") {
        libc::CLONE_DETACHED
    } else {
        0
    };

pub fn apply() {
    if let Err(err) = install(std::process::id()) {
        warning!("Sandbox: the seccomp filter could not be enabled: {err}");
    }
}

// Any call not added below kills the process
fn install(pid: u32) -> Result<(), SeccompError> {
    let mut filter = ScmpFilterContext::new(ScmpAction::KillProcess)?;
    // Also covers the logger thread started before the sandbox
    filter.set_ctl_tsync(true)?;
    // Calls through another ABI, such as 32 bit x86 or x32, kill the process too
    filter.set_act_badarch(ScmpAction::KillProcess)?;
    for &name in ALLOWED.iter().chain(ALLOWED_ARCH) {
        add_rule(&mut filter, name, ScmpAction::Allow, &[])?;
    }
    for &name in FALLBACK {
        add_rule(&mut filter, name, ScmpAction::Errno(libc::ENOSYS), &[])?;
    }
    // Only the terminal checks of isatty, which blocks TIOCSTI input injection
    for request in [libc::TCGETS, libc::TCGETS2, libc::TIOCGWINSZ] {
        add_rule(&mut filter, "ioctl", ScmpAction::Allow, &[low_bits(1, request as u32)])?;
    }
    // Only new threads, never a new process or namespace
    add_rule(
        &mut filter,
        "clone",
        ScmpAction::Allow,
        &[low_bits(0, THREAD_FLAGS as u32)],
    )?;
    // Only signals to this process, which abort() sends
    add_rule(&mut filter, "tgkill", ScmpAction::Allow, &[low_bits(0, pid)])?;
    filter.load()
}

// A name the installed libseccomp does not know cannot be added and stays blocked
fn add_rule(
    filter: &mut ScmpFilterContext,
    name: &str,
    action: ScmpAction,
    args: &[ScmpArgCompare],
) -> Result<(), SeccompError> {
    if let Ok(syscall) = ScmpSyscall::from_name(name) {
        filter.add_rule_conditional(action, syscall, args)?;
    }
    Ok(())
}

// Match the low 32 bits, which hold the whole value of each argument checked here
fn low_bits(index: u32, value: u32) -> ScmpArgCompare {
    ScmpArgCompare::new(index, ScmpCompareOp::MaskedEqual(u32::MAX.into()), value.into())
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::CString,
        io::{BufRead, BufReader},
        os::unix::process::ExitStatusExt,
        path::Path,
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };

    use libc::c_long;

    use super::*;

    const CASE_ENV: &str = "OUCH_TEST_SECCOMP_CASE";
    const TEST_NAME: &str = "sandbox::seccomp::tests::filter_allows_and_blocks_system_calls";
    // Exit code of a child that could not install the filter
    const UNAVAILABLE: i32 = 77;

    fn errno(ret: c_long) -> i32 {
        if ret == -1 {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
        } else {
            0
        }
    }

    // Each case runs in a new process because the filter covers all threads and blocked calls kill
    #[test]
    fn filter_allows_and_blocks_system_calls() {
        if let Ok(case) = std::env::var(CASE_ENV) {
            run_case(&case);
            return;
        }
        let command = |case: &str| {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.args(["--exact", TEST_NAME, "--nocapture"]).env(CASE_ENV, case);
            command
        };
        let run = |case: &str| command(case).status().unwrap();
        let allowed = run("allowed");
        if allowed.code() == Some(UNAVAILABLE) {
            // seccomp is not available here, for example under qemu-user
            return;
        }
        assert!(allowed.success(), "an allowed call failed");
        for case in [
            "early_thread",
            "socket",
            "execve",
            "unshare",
            "ioctl",
            "fork",
            "clone_flags",
            "prctl",
            "tgkill",
        ] {
            assert_eq!(run(case).signal(), Some(libc::SIGSYS), "{case} was not blocked");
        }
        // Crashes keep their own signal, but musl raises SIGABRT with tkill, which stays blocked
        let abort = if cfg!(target_env = "musl") {
            libc::SIGSYS
        } else {
            libc::SIGABRT
        };
        for (case, signal) in [("abort", abort), ("segfault", libc::SIGSEGV)] {
            assert_eq!(run(case).signal(), Some(signal), "{case} ended with the wrong signal");
        }
        // musl 1.2.6 rejects the Rust signal stack on CPUs with large signal frames, leaving overflows unhandled
        let overflow = run("stack_overflow").signal();
        let unhandled = cfg!(target_env = "musl") && overflow == Some(libc::SIGSEGV);
        assert!(
            overflow == Some(abort) || unhandled,
            "stack_overflow ended with signal {overflow:?}"
        );

        // A stopped and resumed process continues its timed waits with restart_syscall
        let mut child = command("stop_resume").stdout(Stdio::piped()).spawn().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
        let started = output.any(|line| line.is_ok_and(|line| line == "ready"));
        assert!(started, "the stop_resume child did not start");
        let pid = child.id() as libc::pid_t;
        std::thread::sleep(Duration::from_millis(200));
        // SAFETY: the signals go to the child started above
        unsafe {
            libc::kill(pid, libc::SIGSTOP);
            std::thread::sleep(Duration::from_millis(200));
            libc::kill(pid, libc::SIGCONT);
        }
        // Read the rest so the child can still print its result
        output.for_each(drop);
        let status = child.wait().unwrap();
        assert!(status.success(), "a stopped and resumed process failed: {status}");
    }

    fn run_case(case: &str) {
        let dir = tempfile::tempdir().unwrap();
        let missing = CString::new("/nonexistent").unwrap();
        let null = std::ptr::null::<libc::c_char>();

        // Started before the filter like the logger thread
        let (start, started) = mpsc::channel::<()>();
        let early_thread = std::thread::spawn(move || {
            if started.recv().is_ok() {
                unsafe { libc::syscall(libc::SYS_socket, libc::AF_UNIX, libc::SOCK_STREAM, 0) };
            }
        });

        // SAFETY: the kernel only reads the byte, the strings and the flags passed here
        unsafe {
            // Blocked calls kill with SIGSYS, which must not leave a core dump
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
            if install(std::process::id()).is_err() {
                std::process::exit(UNAVAILABLE);
            }
            match case {
                "allowed" => check_allowed(dir.path()),
                "early_thread" => {
                    start.send(()).unwrap();
                    early_thread.join().unwrap();
                }
                "socket" => {
                    libc::syscall(libc::SYS_socket, libc::AF_UNIX, libc::SOCK_STREAM, 0);
                }
                "execve" => {
                    libc::syscall(libc::SYS_execve, missing.as_ptr(), null, null);
                }
                "unshare" => {
                    libc::syscall(libc::SYS_unshare, libc::CLONE_NEWUSER);
                }
                "ioctl" => {
                    libc::syscall(libc::SYS_ioctl, -1, libc::TIOCSTI, &0u8 as *const u8);
                }
                "fork" => {
                    if libc::syscall(libc::SYS_clone, libc::SIGCHLD, 0, 0, 0, 0) == 0 {
                        libc::_exit(0);
                    }
                }
                "clone_flags" => {
                    libc::syscall(libc::SYS_clone, libc::CLONE_THREAD, 0, 0, 0, 0);
                }
                "prctl" => {
                    libc::syscall(libc::SYS_prctl, libc::PR_SET_DUMPABLE, 1, 0, 0, 0);
                }
                "tgkill" => {
                    libc::syscall(libc::SYS_tgkill, 1, 1, 0);
                }
                "abort" => std::process::abort(),
                "segfault" => std::ptr::without_provenance_mut::<u8>(8).write_volatile(1),
                "stop_resume" => {
                    println!("ready");
                    let (_keep, wait) = mpsc::channel::<()>();
                    let _ = wait.recv_timeout(Duration::from_secs(1));
                }
                "stack_overflow" => {
                    let thread = std::thread::Builder::new().stack_size(64 * 1024).spawn(|| overflow(0));
                    let _ = thread.unwrap().join();
                }
                _ => unreachable!("unknown case {case}"),
            }
        }
    }

    // Recurse until the stack overflows
    fn overflow(depth: u64) -> u64 {
        let frame = std::hint::black_box([depth; 64]);
        if depth == u64::MAX {
            return 0;
        }
        overflow(depth + 1) + frame[0]
    }

    fn check_allowed(dir: &Path) {
        let null = std::ptr::null::<libc::c_char>();
        // SAFETY: every pointer passed is null
        unsafe {
            for &name in FALLBACK {
                let Ok(syscall) = ScmpSyscall::from_name(name) else {
                    continue;
                };
                let nr = c_long::from(i32::from(syscall));
                assert_eq!(errno(libc::syscall(nr, 0, 0, 0, 0, 0)), libc::ENOSYS, "fallback {name}");
            }
            for request in [libc::TCGETS, libc::TCGETS2, libc::TIOCGWINSZ] {
                assert_eq!(errno(libc::syscall(libc::SYS_ioctl, -1, request, null)), libc::EBADF);
            }
            let own = libc::syscall(libc::SYS_tgkill, libc::getpid(), libc::gettid(), 0);
            assert_eq!(errno(own), 0, "tgkill to this process");
        }

        // Threads and the file operations of extraction keep working
        std::thread::spawn(|| ()).join().unwrap();
        let file = dir.join("file");
        std::fs::write(&file, "data").unwrap();
        std::fs::create_dir(dir.join("sub")).unwrap();
        std::fs::rename(&file, dir.join("sub/file")).unwrap();
        std::fs::hard_link(dir.join("sub/file"), dir.join("link")).unwrap();
        std::os::unix::fs::symlink("sub/file", dir.join("symlink")).unwrap();
        let target = std::fs::canonicalize(dir.join("symlink")).unwrap();
        assert_eq!(std::fs::read_to_string(target).unwrap(), "data");
        assert_eq!(std::fs::read_dir(dir).unwrap().count(), 3);
        std::fs::remove_dir_all(dir.join("sub")).unwrap();
    }
}
