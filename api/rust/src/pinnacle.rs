// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Compositor management.
//!
//! This module provides [`Pinnacle`], which allows you to quit the compositor.

use futures::executor::block_on;
use pinnacle_api_defs::pinnacle::v0alpha1::{
    pinnacle_service_client::PinnacleServiceClient, QuitRequest,
};
use tonic::transport::Channel;

/// A struct that allows you to quit the compositor.
#[derive(Debug, Clone)]
pub struct Pinnacle {
    channel: Channel,
}

impl Pinnacle {
    pub(crate) fn new(channel: Channel) -> Self {
        Self { channel }
    }

    fn create_pinnacle_client(&self) -> PinnacleServiceClient<Channel> {
        PinnacleServiceClient::new(self.channel.clone())
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
        let mut client = self.create_pinnacle_client();
        block_on(client.quit(QuitRequest {})).unwrap();
    }
}
