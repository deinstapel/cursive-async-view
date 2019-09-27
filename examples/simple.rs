extern crate cursive_async_view;

use std::time::{Instant, Duration};

use cursive::{self, views::Dialog, views::Button, Cursive};
use cursive_async_view::{AsyncView, AsyncState};

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let time = Instant::now();
    let async_view = AsyncView::new(&mut siv, move |_siv| {
        if time + Duration::from_secs(5) < Instant::now() {
            AsyncState::Loaded(
                Button::new("Yay! The content has loaded!", |_| {}).disabled()
            )
        } else {
            AsyncState::Pending
        }
    });

    let dialog = Dialog::around(async_view.with_width(40))
        .button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
