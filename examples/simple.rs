extern crate cursive_async_view;

use cursive::{self, Cursive, views::TextView, views::Dialog};
use cursive_async_view::AsyncView;

/*const CAR: &str = "
    ▄█████████████▄
   ▄██▀▀▀▀██▀▀▀▀████▄
  ▄██▀    ██     ▀████▄
 ▄███▄▄▄▄▄██▄▄▄▄▄▄▄█████▄
███████████████████████████▄
████████████████████████████
██████▀▀████████████▀▀██████
▀▀▀██    ██▀▀▀▀▀▀██    ██▀▀▀
   ███▄▄███      ███▄▄███
     ▀▀▀▀          ▀▀▀▀
";
const CAR_WIDTH: usize = 28;

fn get_animation() -> Vec<StyledString> {
    let width = WIDTH + CAR_WIDTH;

    (0..width + 1)
        .map(|x| {
            let ip = if x as f64 <= width as f64 / 2.0 {
                (x as f64 / (width as f64 / 2.0)).circular_out() / 2.0
            } else {
                ((x - width / 2) as f64 / (width as f64 / 2.0)).circular_in() / 2.0 + 1.0.circular_out() / 2.0
            };
            (ip * width as f64) as usize
        })
        .map(|f| {
            let mut result = StyledString::default();
            for line in CAR.lines() {
                if f == 0 || f == width{
                    result.append_plain(format!(
                        "{}\n",
                        repeat_str(" ", WIDTH),
                    ));
                } else if f < CAR_WIDTH {
                    result.append_plain(format!(
                        "{}{}\n",
                        chop::substr(line, CAR_WIDTH - f, 0),
                        repeat_str(" ", width - f - CAR_WIDTH),
                    ));
                } else if f >= WIDTH {
                    result.append_plain(format!(
                        "{}{}\n",
                        repeat_str(" ", f - CAR_WIDTH),
                        chop::substr(line, 0, CAR_WIDTH - (f - WIDTH)),
                    ));
                } else {
                    result.append_plain(format!(
                        "{}{}{}\n",
                        repeat_str(" ", f - CAR_WIDTH),
                        line,
                        repeat_str(" ", width - f - CAR_WIDTH),
                    ));
                }
            }

            result
        })
        .collect::<Vec<_>>()
}*/

fn main() {
    cursive::logger::init();

    let mut siv = Cursive::default();

    // We can quit by pressing `q`
    siv.add_global_callback('q', Cursive::quit);

    let async_view = AsyncView::new(&siv, move ||{
        std::thread::sleep(std::time::Duration::from_secs(10));
        TextView::new("Yay!\n\nThe content has loaded!               ")
    }).with_width(40);

    let dialog = Dialog::around(async_view)
        .button("Ok", |s| s.quit());

    siv.add_layer(dialog);
    siv.run();
}
