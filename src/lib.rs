//! This project provides a wrapper view with a loading screen for
//! [gyscos/cursive](https://github.com/gyscos/cursive) views. The loading screen will
//! disappear once the wrapped view is fully loaded. This is useful for displaying views
//! which may take long to construct or depend on e.g. the network.
//!
//! # Asynchronous view loading without progress information
//!
//! If you can't tell the progress during a long taking creation of a view, you may
//! wrap the creation of this view in an `AsyncView`. This will display a loading
//! animation until the inner view is ready to be drawn.
//!
//! ```
//! use cursive::{views::TextView, Cursive};
//! use cursive_async_view::AsyncView;
//!
//! let mut siv = Cursive::default();
//! let async_view = AsyncView::new(&siv, move || {
//!     std::thread::sleep(std::time::Duration::from_secs(10));
//!     TextView::new("Yay!\n\nThe content has loaded!")
//! });
//!
//! siv.add_layer(async_view);
//! // siv.run();
//! ```
//!
//! # Asynchronous view loading with a progress bar
//!
//! If you have information about the progress a long taking view creation has made,
//! you can wrap the creation in an `AsyncProgressView`. This will display a progress
//! bar until the inner view is ready to be drawn.
//!
//! ```
//! use crossbeam::Sender;
//! use cursive::{views::TextView, Cursive};
//! use cursive_async_view::AsyncProgressView;
//!
//! let mut siv = Cursive::default();
//! let async_view = AsyncProgressView::new(&siv, |s: Sender<f32>| {
//!     std::thread::sleep(std::time::Duration::from_secs(1));
//!     s.send(0.2).unwrap();
//!     std::thread::sleep(std::time::Duration::from_secs(1));
//!     s.send(0.4).unwrap();
//!     std::thread::sleep(std::time::Duration::from_secs(1));
//!     s.send(0.6).unwrap();
//!     std::thread::sleep(std::time::Duration::from_secs(1));
//!     s.send(0.8).unwrap();
//!     std::thread::sleep(std::time::Duration::from_secs(1));
//!     s.send(1.0).unwrap();
//!     TextView::new("Yay, the content has loaded!")
//! });
//!
//! siv.add_layer(async_view);
//! // siv.run();
//! ```

mod infinite;
mod progress;
mod utils;

pub use infinite::{default_animation, AsyncView, AsyncHandle, HandleError, AnimationFrame};
//pub use progress::{default_progress, AsyncProgressView};
