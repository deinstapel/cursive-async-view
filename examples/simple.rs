use std::time::{Duration, Instant};

use cursive::views::{Dialog, TextView};
use cursive::{Cursive, CursiveExt};
use cursive_async_view::{AsyncState, AsyncView};

fn main() {
    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let start_time = Instant::now();
    let async_view = AsyncView::new(&mut siv, move || {
        // This function will be called several times.
        // It should signal `Available` when the view is available for drawing.

        // Do not run heavy calculations here!
        // Instead look if the calculation is ready.
        if start_time.elapsed() > Duration::from_secs(5) {
            // we are ready to display the content
            AsyncState::Available(TextView::new("Krawoombah! Async loading is working 🦀"))
        } else {
            // still waiting for five seconds to pass
            AsyncState::Pending
        }
    });

    // dialogs are cool, so let's use one!
    let dialog = Dialog::around(async_view.with_width(40)).button("Ok", |s| s.quit());
    siv.add_layer(dialog);

    // the loneliness of the long distance runner
    siv.run();
}
