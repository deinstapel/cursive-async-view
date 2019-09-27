extern crate cursive_async_view;

use std::time::{Instant, Duration};

use cursive::{self, views::Dialog, views::Button, Cursive};
use cursive_async_view::{AsyncView, AsyncHandle};
use send_wrapper::SendWrapper;

fn check_view_ready(
    siv: &mut Cursive,
    handle: SendWrapper<AsyncHandle<Button>>,
    time: Instant,
) {
    if time + Duration::from_secs(5) < Instant::now() {
        handle.take().loaded(
            Button::new("Yay! The content has loaded!", |_| {})
                .disabled()
        ).unwrap();
    } else {
        let sink = siv.cb_sink().clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(16));
            sink.send(Box::new(move |siv| check_view_ready(siv, handle, time))).unwrap();
        });
    }
}

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let async_view = AsyncView::new(&siv);
    check_view_ready(&mut siv, async_view.handle(), Instant::now());

    let dialog = Dialog::around(async_view.with_width(40))
        .button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
