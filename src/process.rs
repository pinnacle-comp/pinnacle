use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    os::fd::{IntoRawFd, RawFd},
    process::Stdio,
};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
use tokio::process::Child;
use tracing::warn;

use crate::util::restore_nofile_rlimit;

#[derive(Debug)]
pub struct ProcessState {
    pub system_processes: sysinfo::System,
    spawned: HashMap<u32, Child>,
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

#[derive(Debug)]
pub struct SpawnOutput {
    pub pid: u32,
    pub stdin: Option<RawFd>,
    pub stdout: Option<RawFd>,
    pub stderr: Option<RawFd>,
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
    ) -> Option<SpawnOutput> {
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

        tokio_cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(envs)
            .args(cmd);

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

        let streams = SpawnOutput {
            pid,
            stdin: child
                .stdin
                .take()
                .and_then(|stdin| Some(stdin.into_owned_fd().ok()?.into_raw_fd())),
            stdout: child
                .stdout
                .take()
                .and_then(|stdout| Some(stdout.into_owned_fd().ok()?.into_raw_fd())),
            stderr: child
                .stderr
                .take()
                .and_then(|stderr| Some(stderr.into_owned_fd().ok()?.into_raw_fd())),
        };

        self.spawned.insert(pid, child);
        self.spawned_already.insert(arg0.clone());

        Some(streams)
    }

    pub fn wait_on_spawn(
        &mut self,
        pid: u32,
    ) -> Option<tokio::sync::oneshot::Receiver<Option<WaitOutput>>> {
        let mut child = self.spawned.remove(&pid)?;
        let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let exit_status = child.wait().await.ok().map(|exit| WaitOutput {
                exit_code: exit.code(),
                exit_msg: Some(exit.to_string()),
            });

            oneshot_tx.send(exit_status).unwrap();
        });

        Some(oneshot_rx)
    }
}
