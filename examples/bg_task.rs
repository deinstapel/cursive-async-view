extern crate cursive_async_view;

use std::thread;
use std::time::Duration;

use cursive::views::{Dialog, TextView};
use cursive::Cursive;
use cursive_async_view::AsyncView;

fn main() {
    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let async_view = AsyncView::new_with_bg_creator(&mut siv, move || {
        // this function is executed in a background thread, so we can block
        // here as long as we like
        thread::sleep(Duration::from_secs(5));

        // enough blocking, let's show the content
        Ok("Yeet! It worked 🖖")
    }, TextView::new); // create a text view from the string

    // dialogs are cool, so let's use one!
    let dialog = Dialog::around(async_view.with_width(40)).button("Ok", |s| s.quit());
    siv.add_layer(dialog);

    // run to the hills
    siv.run();
}
