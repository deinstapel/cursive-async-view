extern crate cursive_async_view;

use std::sync::mpsc::channel;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

use cursive::views::{Dialog, LinearLayout, RadioGroup};
use cursive::Cursive;
use cursive_async_view::{AsyncState, AsyncView};

fn main() {
    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    // create channel to send radio buttons text over it
    let (tx, rx) = channel();

    // calculate the radio button contents in a different thread
    thread::spawn(move || {
        // Figuring out the radio button texts takes veeeeery long!
        // Glad we have the async view, entertaining the user until
        // all our figuring out is done!

        tx.send("🐶🔔 Ding dong, you are wrong...").unwrap();
        thread::sleep(Duration::from_secs(1));
        tx.send("🦆💦 Splish splash, your opinion is trash!")
            .unwrap();
        thread::sleep(Duration::from_secs(1));
        tx.send("🦀🛑 Flippity flop, you need to stop").unwrap();
        thread::sleep(Duration::from_secs(1));
        tx.send("🔫🐸 Hippity hoppity, this view is now my property")
            .unwrap();
        thread::sleep(Duration::from_secs(1));
        tx.send("🦄🚬 Miss me").unwrap();
        thread::sleep(Duration::from_secs(1));
    });

    let mut group = RadioGroup::new(); // !Send struct

    // We can't use a LinearLayout here, 'cause it's not `Copy`. Further
    // explanation below.
    let mut buttons = Vec::new();
    let async_view = AsyncView::new(&mut siv, move || {
        // This function will be called several times.
        // It should signal `Available` when the view is available for drawing.

        // Do not run heavy calculations here!
        // Instead look if the calculation is ready.
        match rx.try_recv() {
            // ooooooh, another radio button for me
            Ok(msg) => {
                buttons.push(group.button_str(msg));

                // there are more buttons to be excited about!
                AsyncState::Pending
            }

            // no button today, but maybe next time
            Err(TryRecvError::Empty) => AsyncState::Pending,

            // channel got closed, so looks like these are all my buttons...
            Err(TryRecvError::Disconnected) => {
                // If the channel has disconnected, we have received all radio
                // buttons, so lets resolve the async view.

                match buttons.len() {
                    0 => AsyncState::Error("Buttons could not be loaded".to_string()),
                    1 => AsyncState::Error("There is no choice!".to_string()),
                    _ => {
                        // ==> EXPLANATION <==
                        // We have to create the linear layout here, as it does
                        // not implement the copy trait. As we cannot move a
                        // captured variable out of an FnMut and cannot copy the
                        // linear layout, we have to create it inside the
                        // closure from the buttons vector.
                        let mut layout = LinearLayout::vertical();
                        for button in buttons.drain(..) {
                            layout.add_child(button);
                        }

                        AsyncState::Available(layout)
                    }
                }
            }
        }
    });

    // dialogs are cool, so let's use one!
    let dialog = Dialog::around(async_view.with_width(40)).button("Ok", |s| s.quit());
    siv.add_layer(dialog);

    // fox on the run!
    siv.run();
}
