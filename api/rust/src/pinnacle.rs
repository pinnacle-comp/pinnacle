// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Compositor management.
//!
//! This module provides [`Pinnacle`], which allows you to quit the compositor.

use std::time::Duration;

use pinnacle_api_defs::pinnacle::v0alpha1::{
    pinnacle_service_client::PinnacleServiceClient, PingRequest, QuitRequest,
};
use rand::RngCore;
use tonic::{transport::Channel, Request};

use crate::block_on_tokio;

/// A struct that allows you to quit the compositor.
#[derive(Debug, Clone)]
pub struct Pinnacle {
    client: PinnacleServiceClient<Channel>,
}

impl Pinnacle {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            client: PinnacleServiceClient::new(channel),
        }
    }

    /// Quit Pinnacle.
    ///
    /// # Examples
    ///
    /// ```
    /// // Quits Pinnacle. What else were you expecting?
    /// pinnacle.quit();
    /// ```
    pub fn quit(&self) {
        let mut client = self.client.clone();
        block_on_tokio(client.quit(QuitRequest {})).unwrap();
    }

    pub(super) async fn ping(&self) -> Result<(), String> {
        let mut client = self.client.clone();
        let mut payload = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut payload);
        let mut request = Request::new(PingRequest {
            payload: Some(payload.to_vec()),
        });
        request.set_timeout(Duration::from_secs(10));

        let response = client
            .ping(request)
            .await
            .map_err(|status| status.to_string())?;

        (response.into_inner().payload() == payload)
            .then_some(())
            .ok_or("timed out".to_string())
    }
}
