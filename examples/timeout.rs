use cursive::{Cursive, views::{Dialog, TextView}};
use cursive_async_view::AsyncView;
use std::time::{Duration, Instant};
use std::sync::mpsc::channel;

// To implement a timeout for your creation you can easily spawn a thread in your creator
// function and use a channel to send yourself the created view back to your creator function.
// The `std::time` structs are well usable here to check if our creation has took too long e.g.
// in the case our creation has to wait for some pending operations which are not completed in the near future.


fn main() {
    let mut siv = Cursive::default();
    let loading_view = AsyncView::new(&siv, || {
        let start_time = Instant::now();

        // We want to wait exactly 5 seconds
        let timeout = Duration::from_secs(5);
        // Create channel to send our view over it
        let (sx,rx) = channel();


        // Do some very instensive but important stuff here!
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(20));
            sx.send(TextView::new("Content has loaded!")).unwrap();
        });
        loop {
            match rx.try_recv() {
                Ok(view) => {
                    // Wohoo we received our view
                    return view
                },
                Err(_) => {
                    let current_time = Instant::now();
                    // If our creation takes too long we just abort it
                    if current_time.duration_since(start_time) > timeout {
                        return TextView::new("Oh no, the view has timed out!")
                    }
                },
            }
        }
    }).with_width(40);

    siv.add_layer(Dialog::around(loading_view).button("Ok", |s| {s.quit()}));
    siv.run();
}
