//! Landlock filesystem sandbox backend.

use std::path::Path;

use landlock::{
    ABI, Access, AccessFs, AccessNet, BitFlags, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreated, RulesetCreatedAttr, RulesetError, Scope,
};

use super::SandboxPolicy;
use crate::warning;

// Landlock ABI v6 is required (Linux 6.12).
// It covers filesystem and network rights and IPC scoping.
const REQUIRED_ABI: ABI = ABI::V6;

pub fn apply(policy: &SandboxPolicy) -> bool {
    match install(policy) {
        Ok(()) => true,
        Err(_) => {
            warning!(
                "Sandbox: could not enable Landlock; running without a sandbox. \
                 This requires the Landlock LSM to be enabled and a kernel with ABI v6 (Linux 6.12 or newer)"
            );
            false
        }
    }
}

// Handle filesystem and network rights and deny every IPC scope.
// Require ABI v6 from Linux 6.12 for landlock sandbox.
fn base_ruleset() -> Result<RulesetCreated, RulesetError> {
    Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(REQUIRED_ABI))?
        .handle_access(AccessNet::from_all(REQUIRED_ABI))?
        .scope(Scope::from_all(REQUIRED_ABI))?
        .create()
}

// whether landlock can actually be enforced on this kernel
pub fn is_available() -> bool {
    base_ruleset().is_ok()
}

// Rights an extractor needs inside an output directory.
// It never grants execution or device nodes or device ioctls.
fn write_grant() -> BitFlags<AccessFs> {
    let denied = AccessFs::Execute | AccessFs::IoctlDev | AccessFs::MakeChar | AccessFs::MakeBlock;
    AccessFs::from_all(REQUIRED_ABI) & !denied
}

// Read only rights for input archives. Execution is never granted.
fn read_grant() -> BitFlags<AccessFs> {
    AccessFs::from_read(REQUIRED_ABI) & !AccessFs::Execute
}

fn install(policy: &SandboxPolicy) -> Result<(), RulesetError> {
    let ruleset = base_ruleset()?;
    let ruleset = add_rules(ruleset, policy.read_paths(), read_grant())?;
    let ruleset = add_rules(ruleset, policy.read_write_paths(), write_grant())?;
    let ruleset = add_rules(ruleset, policy.remove_in_paths(), AccessFs::RemoveFile.into())?;
    let ruleset = add_rules(ruleset, policy.remove_dir_in_paths(), AccessFs::RemoveDir.into())?;
    // No network rules and no scope exceptions are added.
    // So nothing can open TCP or reach abstract sockets or signal outside processes.
    let _ = ruleset.restrict_self()?;
    Ok(())
}

// Drop directory only rights on plain files so the file rule is fully enforced.
fn rights_for(access: BitFlags<AccessFs>, is_dir: bool) -> BitFlags<AccessFs> {
    if is_dir {
        access
    } else {
        access & AccessFs::from_file(REQUIRED_ABI)
    }
}

fn add_rules<I>(ruleset: RulesetCreated, paths: I, access: BitFlags<AccessFs>) -> Result<RulesetCreated, RulesetError>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    let mut current = ruleset;
    for path in paths {
        let path = path.as_ref();
        match PathFd::new(path) {
            Ok(fd) => {
                // fstat the opened descriptor so the directory check matches the exact file the
                // rule is built from, instead of a separate stat that could be swapped underneath.
                let is_dir = {
                    use std::os::fd::{AsFd, AsRawFd};
                    let mut st: libc::stat = unsafe { std::mem::zeroed() };
                    // SAFETY: fd is open for this call and st is a valid out-parameter.
                    let ok = unsafe { libc::fstat(fd.as_fd().as_raw_fd(), &mut st) } == 0;
                    ok && (st.st_mode & libc::S_IFMT as libc::mode_t) == libc::S_IFDIR as libc::mode_t
                };
                let granted = rights_for(access, is_dir);
                if granted.is_empty() {
                    continue;
                }
                current = current.add_rule(PathBeneath::new(fd, granted))?;
            }
            Err(err) => {
                warning!("Sandbox: cannot open {}: {err}", path.display());
            }
        }
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_never_allow_execution() {
        assert!(!read_grant().contains(AccessFs::Execute));
        assert!(!write_grant().contains(AccessFs::Execute));
    }

    #[test]
    fn write_grant_denies_devices_and_ioctl() {
        let g = write_grant();
        assert!(!g.contains(AccessFs::IoctlDev));
        assert!(!g.contains(AccessFs::MakeChar));
        assert!(!g.contains(AccessFs::MakeBlock));
    }

    #[test]
    fn write_grant_allows_normal_extraction() {
        let g = write_grant();
        for right in [
            AccessFs::WriteFile,
            AccessFs::MakeReg,
            AccessFs::MakeDir,
            AccessFs::MakeSym,
            AccessFs::Refer,
            AccessFs::Truncate,
        ] {
            assert!(g.contains(right));
        }
    }

    #[test]
    fn read_grant_is_read_only() {
        let g = read_grant();
        assert!(g.contains(AccessFs::ReadFile));
        assert!(g.contains(AccessFs::ReadDir));
        assert!(!g.contains(AccessFs::WriteFile));
        assert!(!g.contains(AccessFs::RemoveFile));
    }

    #[test]
    fn network_access_is_handled() {
        let net = AccessNet::from_all(REQUIRED_ABI);
        assert!(net.contains(AccessNet::BindTcp));
        assert!(net.contains(AccessNet::ConnectTcp));
    }

    #[test]
    fn ipc_scopes_are_handled() {
        let scopes = Scope::from_all(REQUIRED_ABI);
        assert!(scopes.contains(Scope::AbstractUnixSocket));
        assert!(scopes.contains(Scope::Signal));
    }

    // Real enforcement check, run only where the kernel provides Landlock. The ruleset is built
    // with raw syscalls and the forked child runs only async-signal-safe syscalls and never
    // allocates, so forking from the multithreaded test harness cannot deadlock on a held lock.
    #[test]
    fn landlock_denies_writes_outside_granted_dir() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};

        // linux/landlock.h
        const FS_WRITE_FILE: u64 = 1 << 1;
        const FS_MAKE_REG: u64 = 1 << 8;
        const RULE_PATH_BENEATH: libc::c_uint = 1;
        #[repr(C)]
        struct RulesetAttr {
            handled_access_fs: u64,
            handled_access_net: u64,
            scoped: u64,
        }
        #[repr(C, packed)]
        struct PathBeneathAttr {
            allowed_access: u64,
            parent_fd: i32,
        }

        let granted = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let inside_c = CString::new(granted.path().join("inside.txt").as_os_str().as_bytes()).unwrap();
        let outside_c = CString::new(outside.path().join("escape.txt").as_os_str().as_bytes()).unwrap();
        let dir_c = CString::new(granted.path().as_os_str().as_bytes()).unwrap();

        let access = FS_WRITE_FILE | FS_MAKE_REG;
        let attr = RulesetAttr {
            handled_access_fs: access,
            handled_access_net: 0,
            scoped: 0,
        };
        // SAFETY: attr is a valid, fully-initialized ruleset descriptor.
        let ruleset_fd = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                &attr as *const RulesetAttr,
                std::mem::size_of::<RulesetAttr>(),
                0u32,
            )
        };
        if ruleset_fd < 0 {
            // Landlock is unavailable on this kernel; there is nothing to enforce.
            return;
        }
        let ruleset_fd = ruleset_fd as libc::c_int;

        // SAFETY: dir_c is a valid path; O_PATH yields a descriptor usable as a rule parent.
        let dir_fd = unsafe { libc::open(dir_c.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
        assert!(dir_fd >= 0, "failed to open the granted directory");
        let beneath = PathBeneathAttr {
            allowed_access: access,
            parent_fd: dir_fd,
        };
        // SAFETY: beneath references the open dir_fd and a valid access mask.
        let added = unsafe {
            libc::syscall(
                libc::SYS_landlock_add_rule,
                ruleset_fd,
                RULE_PATH_BENEATH,
                &beneath as *const PathBeneathAttr,
                0u32,
            )
        };
        assert_eq!(added, 0, "failed to add the path rule");

        // SAFETY: the child runs only async-signal-safe syscalls and never allocates before _exit,
        // so forking from a possibly-multithreaded harness is safe here.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            unsafe {
                libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0);
                let restricted = libc::syscall(libc::SYS_landlock_restrict_self, ruleset_fd, 0u32);
                let inside = libc::open(inside_c.as_ptr(), libc::O_CREAT | libc::O_WRONLY, 0o600);
                let outside = libc::open(outside_c.as_ptr(), libc::O_CREAT | libc::O_WRONLY, 0o600);
                let ok = restricted == 0 && inside >= 0 && outside < 0;
                libc::_exit(if ok { 0 } else { 1 });
            }
        }
        assert!(pid > 0, "fork failed");
        let mut status = 0;
        // SAFETY: status is a valid out-parameter for the child just forked.
        unsafe { libc::waitpid(pid, &mut status, 0) };
        assert!(libc::WIFEXITED(status), "the sandboxed child did not exit normally");
        assert_eq!(
            libc::WEXITSTATUS(status),
            0,
            "a write outside the granted directory was not denied by Landlock"
        );
    }
}
