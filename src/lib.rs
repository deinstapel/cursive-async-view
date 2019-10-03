//! This project provides a wrapper view with a loading screen for
//! [gyscos/cursive](https://github.com/gyscos/cursive) views. The loading screen
//! will disappear once the wrapped view is fully loaded. This is useful for
//! displaying views which need data that takes long to construct or depend on
//! e.g. the network.
//!
//! # Asynchronous view loading without progress information
//!
//! If you can't tell the progress during a long taking preparation of data for
//! a view, you may wrap the creation of this view in an `AsyncView`. This will
//! display a loading animation until the inner view is ready to be drawn.
//!
//! ```
//! use std::time::{Instant, Duration};
//! use cursive::{views::TextView, Cursive};
//! use cursive_async_view::{AsyncView, AsyncState};
//!
//! let mut siv = Cursive::default();
//! let instant = Instant::now();
//! let async_view = AsyncView::new(&mut siv, move || {
//!     if instant.elapsed() > Duration::from_secs(10) {
//!         AsyncState::Available(
//!             TextView::new("Yay!\n\nThe content has loaded!")
//!         )
//!     } else {
//!         AsyncState::Pending
//!     }
//! });
//!
//! siv.add_layer(async_view);
//! // siv.run();
//! ```
//!
//! Refer to the `AsyncView` struct level documentation for a detailed
//! explanation or to the `simple` example located in the source code
//! repository.
//!
//! # Asynchronous view loading with a progress bar
//!
//! If you have information about the progress a long taking view creation has made,
//! you can wrap the creation in an `AsyncProgressView`. This will display a progress
//! bar until the inner view is ready to be drawn.
//!
//! ```
//! use cursive::{views::TextView, Cursive};
//! use cursive_async_view::{AsyncProgressView, AsyncProgressState};
//!
//! let mut siv = Cursive::default();
//! let start = std::time::Instant::now();
//! let async_view = AsyncProgressView::new(&mut siv, move || {
//!     if start.elapsed().as_secs() < 3 {
//!         AsyncProgressState::Pending(start.elapsed().as_secs() as f32 / 3f32)
//!     } else {
//!         AsyncProgressState::Available(TextView::new("Finally it loaded!"))
//!     }
//! });
//!
//! siv.add_layer(async_view);
//! // siv.run();
//! ```

mod infinite;
mod progress;
mod utils;

pub use infinite::{default_animation, default_error, AnimationFrame, AsyncState, AsyncView};
pub use progress::{
    default_progress, default_progress_error, AsyncProgressState, AsyncProgressView, AnimationProgressFrame
};
