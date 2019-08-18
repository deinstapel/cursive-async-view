use cursive::{
    views::{Dialog, TextView},
    Cursive,
};
use cursive_async_view::AsyncView;
use std::time::Duration;

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let async_view = AsyncView::new(&siv, move || {
        std::thread::sleep(std::time::Duration::from_secs(10));
        TextView::new("Yay!\n\nThe content has loaded!               ")
    })
    .with_width(40)
    .with_timeout(Duration::from_secs(5))
    .with_timeout_view(TextView::new("Oh no, the content has not loaded :("));

    let dialog = Dialog::around(async_view).button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
