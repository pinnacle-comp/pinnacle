use std::{env, fs::File, io::Write, os::fd::FromRawFd};

use tracing::warn;

use crate::config::GRPC_SOCKET_ENV;

pub fn import_environment() {
    let variables = [
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_TYPE",
        GRPC_SOCKET_ENV,
        // TODO:
        // #[cfg(feature = "snowcap")]
        // "SNOWCAP_GRPC_SOCKET",
    ]
    .join(" ");

    let init_system_import = format!("systemctl --user import-environment {variables};");

    let res = std::process::Command::new("/bin/sh")
        .args([
            "-c",
            &format!(
                "{init_system_import}\
                hash dbus-update-activation-environment 2>/dev/null && \
                dbus-update-activation-environment {variables}"
            ),
        ])
        .spawn();

    // Wait for the import process to complete, otherwise services will start too fast without
    // environment variables available.
    match res {
        Ok(mut child) => match child.wait() {
            Ok(status) => {
                if !status.success() {
                    warn!("Import environment shell exited with {status}");
                }
            }
            Err(err) => {
                warn!("Error waiting for import environment shell: {err}");
            }
        },
        Err(err) => {
            warn!("Error spawning shell to import environment: {err}");
        }
    }
}

pub fn notify_fd() -> anyhow::Result<()> {
    let fd = match env::var("NOTIFY_FD") {
        Ok(notify_fd) => notify_fd.parse()?,
        Err(env::VarError::NotPresent) => return Ok(()),
        Err(err) => return Err(err.into()),
    };
    env::remove_var("NOTIFY_FD");
    let mut notif = unsafe { File::from_raw_fd(fd) };
    notif.write_all(b"READY=1\n")?;
    Ok(())
}
