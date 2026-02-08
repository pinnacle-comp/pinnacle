//! Experimental APIs.
//!
//! IMPORTANT: These are unstable and may change at any moment.

#[cfg(feature = "snowcap")]
pub use snowcap_api;

/// Input grabbing.
#[cfg(feature = "snowcap")]
pub mod input_grab {
    use snowcap_api::{
        input::Modifiers,
        layer::LayerHandle,
        widget::{Program, row::Row},
    };
    use xkbcommon::xkb::Keysym;

    struct InputGrab;

    impl Program for InputGrab {
        type Message = ();

        fn update(&mut self, _msg: Self::Message) {}

        fn view(&self) -> Option<snowcap_api::widget::WidgetDef<Self::Message>> {
            Some(
                Row::new()
                    .width(snowcap_api::widget::Length::Fixed(1.0))
                    .height(snowcap_api::widget::Length::Fixed(1.0))
                    .into(),
            )
        }
    }

    /// A handle to an input grab.
    pub struct InputGrabber(LayerHandle<()>);

    impl InputGrabber {
        /// Stops this input grab.
        pub fn stop(&self) {
            self.0.close();
        }
    }

    /// Grabs keyboard input.
    ///
    /// All keyboard input will be redirected to this grabber (assuming another exclusive layer
    /// surface doesn't open). Keybinds will still work.
    ///
    /// Don't forget to add a way to close the grabber, or else input will be grabbed forever!
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pinnacle_api::experimental::input_grab;
    /// # use pinnacle_api::Keysym;
    /// input_grab::grab_input(|grabber, key, mods| {
    ///     if key == Keysym::e {
    ///         println!("An `e` was pressed!");
    ///     }
    ///
    ///     if key == Keysym::Escape {
    ///         grabber.stop();
    ///     }
    /// });
    /// ```
    pub fn grab_input<F>(mut with_input: F)
    where
        F: FnMut(InputGrabber, Keysym, Modifiers) + Send + 'static,
    {
        let grabber = snowcap_api::layer::new_widget(
            InputGrab,
            None,
            snowcap_api::layer::KeyboardInteractivity::Exclusive,
            snowcap_api::layer::ExclusiveZone::Respect,
            snowcap_api::layer::ZLayer::Overlay,
        );

        let grabber = match grabber {
            Ok(grabber) => grabber,
            Err(err) => {
                println!("ERROR: failed to grab input: {err}");
                return;
            }
        };

        grabber.on_key_press(move |this, key, mods| {
            with_input(InputGrabber(this), key, mods);
        });
    }
}
