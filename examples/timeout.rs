use cursive::{
    views::{Dialog, TextView},
    Cursive,
};
use cursive_async_view::{AsyncState, AsyncView};
use std::sync::mpsc::{channel, TryRecvError};
use std::time::{Duration, Instant};

fn main() {
    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    // remember when we started calculation, to check for the timeout
    let start_time = Instant::now();

    // create channel to send our view over it
    let (tx, rx) = channel();

    // do some very instensive but important stuff here!
    std::thread::spawn(move || {
        // calc calc calc
        std::thread::sleep(Duration::from_secs(20));

        // as TextView implements `Send` we can send it between threads :)
        tx.send(TextView::new("Content has loaded!")).ok();
    });

    let loading_view = AsyncView::new(&mut siv, move || {
        // we want to inform that something is wrong after 5 seconds
        if start_time.elapsed() > Duration::from_secs(5) {
            // ideally, we should stop the calc thread here, but meh...
            AsyncState::Error("Oh no, the view has timed out!".to_string())
        } else {
            // let's see if the view is available
            match rx.try_recv() {
                // yeet, it's there!
                Ok(view) => AsyncState::Available(view),

                // duh, still pending. Let's try another time...
                Err(TryRecvError::Empty) => AsyncState::Pending,

                // noooooo, my mighty channel
                Err(TryRecvError::Disconnected) => {
                    AsyncState::Error("Shoot, view creation thread exited...".to_string())
                }
            }
        }
    })
    .with_width(40);

    // be fancy, add a dialog!
    siv.add_layer(Dialog::around(loading_view).button("Ok", |s| s.quit()));

    // run Forest, run!!
    siv.run();
}
