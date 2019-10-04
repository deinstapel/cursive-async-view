extern crate cursive_async_view;

use std::time::{Instant, Duration};
use cursive::{self, views::Dialog, views::TextView, Cursive};
use cursive_async_view::{AsyncProgressState, AsyncProgressView};

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let start = Instant::now();

    let async_view = AsyncProgressView::new(&mut siv, move || {
        if start.elapsed() > Duration::from_secs(5) {
            AsyncProgressState::Available(TextView::new("🦀 Yay, the content has loaded! 🦀"))
        } else {
            AsyncProgressState::Pending(1f32 / 5f32 * start.elapsed().as_secs() as f32)
        }
    })
    .with_width(40);

    let dialog = Dialog::around(async_view).button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
