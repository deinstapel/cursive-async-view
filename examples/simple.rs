extern crate cursive_async_view;

use std::sync::mpsc::channel;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

use cursive::{self, views::Dialog, views::Button, Cursive};
use cursive_async_view::{AsyncView, AsyncState};

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let (tx, rx) = channel();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(5));
        tx.send("Yay! The content has loaded!").unwrap();
    });

    let async_view = AsyncView::new(&mut siv, move || {
        // This function will be called several times.
        // It should signal `Loaded` when the view is available for drawing.

        // Do not run heavy calculations here!
        // Instead look if the calculation is ready.
        match rx.try_recv() {
            Ok(msg) => AsyncState::Loaded(Button::new(msg, |_| {}).disabled()),
            Err(TryRecvError::Empty) => AsyncState::Pending,
            Err(TryRecvError::Disconnected) => AsyncState::Error("Data not available".to_string()),
        }
    });

    let dialog = Dialog::around(async_view.with_width(40))
        .button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
