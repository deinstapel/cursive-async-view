use std::sync::mpsc::{self, Receiver};
use cursive::{wrap_impl, Cursive, Printer, Rect, Vec2};
use cursive::event::{Event, EventResult, AnyCb};
use cursive::direction::Direction;
use cursive::view::{View, ViewWrapper, Selector};
use cursive::views::{TextView};
use cursive::utils::markup::StyledString;
use interpolation::Ease;
use voca_rs::chop;

/// Repeat the string `s` `n` times by concatenating.
pub fn repeat_str<S: Into<String> + Clone>(s: S, n: usize) -> String {
    std::iter::repeat(s.into()).take(n).collect::<String>()
}

fn default_animation(total_width: usize) -> Vec<StyledString> {
    let mut frames = Vec::new();
    let duration = 4 * 1000 / 30;
    let durationf = duration as f64;

    for idx in 0..duration + 1 {
        let idxf = idx as f64;
        let f = if idxf <= durationf / 2.0 {
            (idxf / (durationf / 2.0)).quartic_in()
        } else {
            ((durationf - idxf) / (durationf / 2.0)).quartic_in()
        };

        let w = (f * total_width as f64 / 3.0) as usize;
        let bar = format!(
            "╭{}╮\n╰{}╯",
            repeat_str("─", w),
            repeat_str("─", w),
        );

        let mut result = StyledString::default();
        let bar_width = w + 2;
        let width = total_width + bar_width;

        let g = (idxf / durationf).circular_in_out() * 2.0 % 1.0;
        let pos = (g * width as f64) as usize;

        for line in bar.lines() {
            if pos == 0 || pos == width {
                result.append_plain(format!(
                    "{}\n",
                    repeat_str(" ", total_width),
                ));
            } else if pos < bar_width {
                result.append_plain(format!(
                    "{}{}\n",
                    chop::substr(line, bar_width - pos, 0),
                    repeat_str(" ", width - pos - bar_width),
                ));
            } else if pos >= total_width {
                result.append_plain(format!(
                    "{}{}\n",
                    repeat_str(" ", pos - bar_width),
                    chop::substr(line, 0, bar_width - (pos - total_width)),
                ));
            } else {
                result.append_plain(format!(
                    "{}{}{}\n",
                    repeat_str(" ", pos - bar_width),
                    line,
                    repeat_str(" ", width - pos - bar_width),
                ));
            }
        }

        frames.push(result);
    }

    frames
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
    animation_fn: Box<dyn Fn(usize) -> Vec<StyledString>>,
    width: Option<usize>,
    pos: usize,
    rx: Receiver<T>,
}

#[derive(Default)]
pub struct AsyncViewBuilder {
    animation_fn: Option<Box<dyn Fn(usize) -> Vec<StyledString>>>,
    width: Option<usize>,
}

impl AsyncViewBuilder {
    pub fn animation_fn<VALUE: Into<Box<dyn Fn(usize) -> Vec<StyledString>>>>(
        self,
        value: VALUE,
    ) -> Self {
        let mut new = self;
        new.animation_fn = Some(value.into());
        new
    }

    pub fn width<VALUE: Into<usize>>(self, value: VALUE) -> Self {
        let mut new = self;
        new.width = Some(value.into());
        new
    }

    pub fn build<F, T: View + Send>(self, siv: &Cursive, creator: F) -> AsyncView<T>
    where
        F: FnOnce() -> T + Send + 'static
    {
        AsyncView::new(
            siv, creator,
            self.width,
            self.animation_fn.unwrap_or(Box::new(default_animation)),
        )
    }
}

impl<T: View + Send> AsyncView<T> {
    // TODO: add timeout parameter
    fn new<F>(
        siv: &Cursive,
        creator: F,
        width: Option<usize>,
        animation_fn: Box<dyn Fn(usize) -> Vec<StyledString>>,
    ) -> Self
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
        let animation = if let Some(width) = width {
            animation_fn(width)
        } else {
            Vec::new()
        };

        Self {
            view: None,
            loading: TextView::new(""),
            animation,
            animation_fn,
            width,
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

            if self.width.is_none() {
                self.animation = (self.animation_fn)(constraint.x);
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
