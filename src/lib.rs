use std::sync::mpsc::{self, Receiver};
use cursive::{wrap_impl, Cursive, Printer, Rect, Vec2};
use cursive::event::{Event, EventResult, AnyCb};
use cursive::direction::Direction;
use cursive::view::{View, ViewWrapper, Selector};
use cursive::views::{TextView};
use cursive::utils::markup::StyledString;
use interpolation::Ease;
use voca_rs::chop;

const CAR: &str = "
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
const WIDTH: usize = 80;

/// Repeat the string `s` `n` times by concatenating.
pub fn repeat_str<S: Into<String> + Clone>(s: S, n: usize) -> String {
    std::iter::repeat(s.into()).take(n).collect::<String>()
}

fn get_animation() -> Vec<StyledString> {
    let width = WIDTH + CAR_WIDTH;

    (0..width + 1)
        .map(|x| {
            let ip = if x as f64 <= width as f64 / 2.0 {
                (x as f64 / (width as f64 / 2.0)).bounce_out() / 2.0
            } else {
                ((x - width / 2) as f64 / (width as f64 / 2.0)).bounce_in() / 2.0 + 1.0.quintic_out() / 2.0
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
}

pub struct DelayView<T: View> {
    view: T,
}

impl<T: View> DelayView<T> {
    pub fn new(view: T, delay: u64) -> Self {
        std::thread::sleep(std::time::Duration::from_secs(delay));

        Self {
            view,
        }
    }
}

impl<T: View> ViewWrapper for DelayView<T> {
    wrap_impl!(self.view: T);
}

pub struct AsyncView<T: View + Send> {
    view: Option<T>,
    loading: TextView,
    animation: Vec<StyledString>,
    pos: usize,
    rx: Receiver<T>,
}

impl<T: View + Send> AsyncView<T> {
    // TODO: add timeout parameter
    pub fn new<F>(siv: &Cursive, creator: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static
    {
        // trust me, I'm an engineer
        let sink = siv.cb_sink().clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            tx.send(creator()).unwrap();
            sink.send(Box::new(|_: &mut Cursive| {}))
        });

        Self {
            view: None,
            loading: TextView::new(""),
            animation: get_animation(),
            pos: 0,
            rx,
        }
    }
}

impl<T: View + Send + Sized> View for AsyncView<T> {
    fn draw(&self, printer: &Printer) {
        match self.view {
            Some(ref view) => view.draw(printer),
            None => self.loading.draw(printer),
        }
    }

    fn layout(&mut self, vec: Vec2) {
        self.loading.set_content(self.animation[self.pos].clone());
        self.pos += 1;
        if self.pos >= self.animation.len() {
            self.pos = 0;
        }

        match self.view {
            Some(ref mut view) => view.layout(vec),
            None => self.loading.layout(vec),
        }
    }

    fn needs_relayout(&self) -> bool {
        if self.view.is_none() {
            return true;
        }

        match self.view {
            Some(ref view) => view.needs_relayout(),
            None => self.loading.needs_relayout(),
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        if self.view.is_none() {
            match self.rx.try_recv() {
                Ok(view) => self.view = Some(view),
                Err(_) => {},
            }
        }

        match self.view {
            Some(ref mut view) => view.required_size(constraint),
            None => self.loading.required_size(constraint),
        }
    }

    fn on_event(&mut self, ev: Event) -> EventResult {
        match self.view {
            Some(ref mut view) => view.on_event(ev),
            None => self.loading.on_event(ev),
        }
    }

    fn call_on_any<'a>(&mut self, sel: &Selector, cb: AnyCb<'a>) {
        match self.view {
            Some(ref mut view) => view.call_on_any(sel, cb),
            None => self.loading.call_on_any(sel, cb),
        }
    }

    fn focus_view(&mut self, sel: &Selector) -> Result<(), ()> {
        match self.view {
            Some(ref mut view) => view.focus_view(sel),
            None => self.loading.focus_view(sel),
        }
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match self.view {
            Some(ref mut view) => view.take_focus(source),
            None => self.loading.take_focus(source),
        }
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        match self.view {
            Some(ref view) => view.important_area(view_size),
            None => self.loading.important_area(view_size),
        }
    }
}
