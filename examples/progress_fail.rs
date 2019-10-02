extern crate cursive_async_view;

use cursive::{self, views::Dialog, views::TextView, Cursive};
use cursive_async_view::{AsyncProgressState, AsyncProgressView};

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let start = std::time::Instant::now();

    let async_view = AsyncProgressView::new(&mut siv, move || {
        if start.elapsed().as_secs() > 2 {
            AsyncProgressState::Error("Oh no, the view could not be loaded!".to_string())
        } else if start.elapsed().as_secs() < 7 {
            AsyncProgressState::Pending(1f32 / 7f32 * start.elapsed().as_secs() as f32)
        } else {
            AsyncProgressState::Available(TextView::new("Yay, the content has loaded!"))
        }
    })
    .with_width(40);

    let dialog = Dialog::around(async_view).button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
