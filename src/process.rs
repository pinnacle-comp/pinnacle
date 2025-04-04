use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
};

use passfd::FdPassingExt;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
use tokio::sync::oneshot;
use tracing::warn;
use xdg::BaseDirectories;

use crate::util::restore_nofile_rlimit;

pub static REMOVE_RUST_BACKTRACE: AtomicBool = AtomicBool::new(false);
pub static REMOVE_RUST_LIB_BACKTRACE: AtomicBool = AtomicBool::new(false);

fn fd_socket_name(pid: u32) -> String {
    format!("pinnacle-fd-{pid}.sock")
}

#[derive(Debug, Clone, Default)]
pub struct ExitInfo {
    pub exit_code: Option<i32>,
    pub exit_msg: Option<String>,
}

#[derive(Debug)]
pub struct ProcessState {
    pub system_processes: sysinfo::System,
    spawned: HashMap<u32, tokio::sync::oneshot::Receiver<ExitInfo>>,
    spawned_already: HashSet<String>,
}

impl ProcessState {
    pub fn new(system: sysinfo::System) -> Self {
        Self {
            system_processes: system,
            spawned: Default::default(),
            spawned_already: Default::default(),
        }
    }
}

pub struct SpawnData {
    pub pid: u32,
    pub fd_socket_path: String,
    pub has_stdin: bool,
    pub has_stdout: bool,
    pub has_stderr: bool,
}

#[derive(Debug)]
pub struct WaitOutput {
    pub exit_code: Option<i32>,
    pub exit_msg: Option<String>,
}

impl ProcessState {
    pub fn spawn(
        &mut self,
        cmd: &[String],
        shell_cmd: &[String],
        unique: bool,
        once: bool,
        envs: HashMap<String, String>,
        base_dirs: &BaseDirectories,
        pipe_processes: bool,
    ) -> Option<SpawnData> {
        let arg0 = cmd.first()?.to_string();

        if once && self.spawned_already.contains(&arg0) {
            return None;
        }

        if unique {
            self.system_processes.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing(),
            );

            let compositor_pid = std::process::id();
            let already_running = self
                .system_processes
                .processes_by_exact_name(OsString::from(arg0.as_str()).as_os_str())
                .any(|proc| {
                    proc.parent()
                        .is_some_and(|parent_pid| parent_pid.as_u32() == compositor_pid)
                });

            if already_running {
                return None;
            }
        }

        let mut cmd = shell_cmd.iter().chain(cmd.iter());
        let program = cmd.next()?;

        let mut tokio_cmd = tokio::process::Command::new(OsString::from(program));

        tokio_cmd.envs(envs).args(cmd);

        if pipe_processes {
            tokio_cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        }

        if REMOVE_RUST_BACKTRACE.load(Ordering::Relaxed) {
            tokio_cmd.env_remove("RUST_BACKTRACE");
        }
        if REMOVE_RUST_LIB_BACKTRACE.load(Ordering::Relaxed) {
            tokio_cmd.env_remove("RUST_LIB_BACKTRACE");
        }

        unsafe {
            tokio_cmd.pre_exec(|| {
                restore_nofile_rlimit();
                Ok(())
            });
        }

        let Ok(mut child) = tokio_cmd.spawn() else {
            warn!("Tried to run {arg0}, but it doesn't exist");
            return None;
        };

        let pid = child.id().expect("child has not polled to completion");

        let socket_dir = base_dirs
            .get_runtime_directory()
            .expect("XDG_RUNTIME_DIR is not set");
        let socket_path = socket_dir.join(fd_socket_name(pid));
        let socket_path_str = socket_path.to_string_lossy().to_string();

        let listener = tokio::net::UnixListener::bind(&socket_path).unwrap(); // TODO: unwrap
        let stdin_fd = child
            .stdin
            .take()
            .map(|stdin| stdin.into_owned_fd().unwrap().into_raw_fd());
        let stdout_fd = child
            .stdout
            .take()
            .map(|stdout| stdout.into_owned_fd().unwrap().into_raw_fd());
        let stderr_fd = child
            .stderr
            .take()
            .map(|stderr| stderr.into_owned_fd().unwrap().into_raw_fd());

        let data = SpawnData {
            pid,
            fd_socket_path: socket_path_str,
            has_stdin: stdin_fd.is_some(),
            has_stdout: stdout_fd.is_some(),
            has_stderr: stderr_fd.is_some(),
        };

        tokio::spawn(async move {
            let Ok((stream, _addr)) = listener.accept().await else {
                return;
            };

            // We only close our copy of the stdin fd so it can get EOF when the config closes
            // its copy. If we do that for stdout and stderr we get SIGPIPEs everywhere.
            if let Some(stdin_fd) = stdin_fd {
                let _ = stream.as_raw_fd().send_fd(stdin_fd);
                // SAFETY: The `send_fd` above dups the fd into the config process,
                // so we are good to reclaim this fd to close it
                unsafe { drop(OwnedFd::from_raw_fd(stdin_fd)) }
            }
            if let Some(stdout_fd) = stdout_fd {
                let _ = stream.as_raw_fd().send_fd(stdout_fd);
            }
            if let Some(stderr_fd) = stderr_fd {
                let _ = stream.as_raw_fd().send_fd(stderr_fd);
            }

            let _ = std::fs::remove_file(socket_path);
        });

        let (oneshot_send, oneshot_recv) = oneshot::channel();

        tokio::spawn(async move {
            let exit_status = child.wait().await;
            let exit_info = exit_status
                .map(|status| ExitInfo {
                    exit_code: status.code(),
                    exit_msg: Some(status.to_string()),
                })
                .unwrap_or_default();
            oneshot_send.send(exit_info).unwrap();
        });

        self.spawned.insert(pid, oneshot_recv);
        self.spawned_already.insert(arg0.clone());

        Some(data)
    }

    pub fn wait_on_spawn(
        &mut self,
        pid: u32,
    ) -> Option<tokio::sync::oneshot::Receiver<Option<ExitInfo>>> {
        let recv = self.spawned.remove(&pid)?;
        let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let exit_status = recv.await.ok();
            oneshot_tx.send(exit_status).unwrap();
        });

        Some(oneshot_rx)
    }
}
