extern crate cursive_async_view;

use cursive::{self, Cursive, views::TextView, views::Dialog};
use cursive_async_view::{AsyncProgressView};
use crossbeam::channel::Sender;

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let async_view = AsyncProgressView::new(&siv, |s: Sender<f32>| {
        std::thread::sleep(std::time::Duration::from_secs(1));
        s.send(0.2).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        s.send(0.4).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        s.send(0.6).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        s.send(0.8).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        s.send(1.0).unwrap();
        TextView::new("Yay, the content has loaded!")
    }).with_width(40);

    let dialog = Dialog::around(async_view)
        .button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
