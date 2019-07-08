extern crate cursive_async_view;

use cursive::{self, Cursive, views::TextView};
use cursive_async_view::{DelayView, AsyncView};

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let view = AsyncView::new(&siv, || {
        DelayView::new(TextView::new("Content loaded!"), 5)
    });

    siv.add_fullscreen_layer(view);
    siv.run();
}
