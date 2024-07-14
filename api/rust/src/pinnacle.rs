// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Compositor management.
//!
//! This module provides [`Pinnacle`], which allows you to quit the compositor.

use std::time::Duration;

use pinnacle_api_defs::pinnacle::v0alpha1::{
    PingRequest, QuitRequest, ReloadConfigRequest, ShutdownWatchRequest, ShutdownWatchResponse,
};
use rand::RngCore;
use tonic::{Request, Streaming};

use crate::{block_on_tokio, pinnacle};

/// A struct that allows you to quit the compositor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Pinnacle;

impl Pinnacle {
    /// Quit Pinnacle.
    ///
    /// # Examples
    ///
    /// ```
    /// // Quits Pinnacle. What else were you expecting?
    /// pinnacle.quit();
    /// ```
    pub fn quit(&self) {
        // Ignore errors here, the config is meant to be killed
        let _ = block_on_tokio(pinnacle().quit(QuitRequest {}));
    }

    /// Reload the currently active config.
    pub fn reload_config(&self) {
        // Ignore errors here, the config is meant to be killed
        let _ = block_on_tokio(pinnacle().reload_config(ReloadConfigRequest {}));
    }

    pub(crate) async fn shutdown_watch(&self) -> Streaming<ShutdownWatchResponse> {
        pinnacle()
            .shutdown_watch(ShutdownWatchRequest {})
            .await
            .unwrap()
            .into_inner()
    }

    /// TODO: eval if this is necessary
    #[allow(dead_code)]
    pub(super) async fn ping(&self) -> Result<(), String> {
        let mut payload = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut payload);
        let mut request = Request::new(PingRequest {
            payload: Some(payload.to_vec()),
        });
        request.set_timeout(Duration::from_secs(10));

        let response = pinnacle()
            .ping(request)
            .await
            .map_err(|status| status.to_string())?;

        (response.into_inner().payload() == payload)
            .then_some(())
            .ok_or("timed out".to_string())
    }
}
