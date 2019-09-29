use notify_rust::{Notification, Timeout};
use cursive::Cursive;
use cursive::views::Dialog;
use cursive_async_view::{AsyncView, AsyncState};

fn main() {
    let mut siv = Cursive::default();
    let start = std::time::Instant::now();
    let async_view = AsyncView::new(&mut siv, move || {
        if start.elapsed().as_secs() < 5 {
            AsyncState::Pending
        } else {
            Notification::new()
                .summary("View ready")
                .body("The view has been successfully loaded!")
                .timeout(Timeout::Milliseconds(5000))
                .icon("terminal")
                .show().expect("Notification could not be displayed");
            AsyncState::Loaded(cursive::views::TextView::new("Content loaded!"))
        }
    });
    let dialog = Dialog::around(async_view.with_width(40))
        .button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
