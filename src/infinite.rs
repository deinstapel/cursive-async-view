use std::thread;
use std::time::Duration;

use cursive::{Cursive, Printer, Rect, Vec2};
use cursive::event::{Event, EventResult, AnyCb};
use cursive::direction::Direction;
use cursive::view::{View, Selector};
use cursive::views::{TextView};
use cursive::utils::markup::StyledString;
use cursive::theme::PaletteColor;
use interpolation::Ease;
use crossbeam::channel::{self, Receiver};
use num::clamp;

use crate::utils;

pub struct AnimationFrame {
    content: StyledString,
    next_frame_idx: usize,
}

pub fn default_animation(width: usize, _height: usize, frame_idx: usize) -> AnimationFrame {
    let foreground = PaletteColor::Highlight;
    let background = PaletteColor::HighlightInactive;
    let symbol = "━";

    let duration = 2 * 1000 / 30;
    let durationf = duration as f64;

    let idx = frame_idx % duration;
    let idxf = idx as f64;
    let factor = idxf / durationf;
    let begin_factor = clamp(((factor + 0.5) % 1.0).circular_in_out(), 0.0, 1.0);
    let end_factor = clamp(((factor + 0.75) % 1.0).circular_in_out() * 2.0, 0.0, 1.0);
    let begin = (begin_factor * width as f64) as usize;
    let end = (end_factor * width as f64) as usize;

    let mut result = StyledString::default();
    if end >= begin {
        result.append_styled(utils::repeat_str(symbol, begin), background);
        result.append_styled(utils::repeat_str(symbol, end - begin), foreground);
        result.append_styled(utils::repeat_str(symbol, width - end), background);
    } else {
        result.append_styled(utils::repeat_str(symbol, end), foreground);
        result.append_styled(utils::repeat_str(symbol, begin - end), background);
        result.append_styled(utils::repeat_str(symbol, width - begin), foreground);
    }

    AnimationFrame {
        content: result,
        next_frame_idx: (idx + 1) % duration,
    }
}

pub struct AsyncView<T: View + Send> {
    view: Option<T>,
    loading: TextView,
    animation_fn: Box<dyn Fn(usize, usize, usize) -> AnimationFrame + 'static>,
    width: Option<usize>,
    height: Option<usize>,
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
        let (tx, rx) = channel::unbounded();
        let (update_tx, update_rx) = channel::unbounded();

        // creation thread for async view
        thread::Builder::new()
            .name(format!("cursive-async-view::creator"))
            .spawn(move || {
                tx.send(creator()).unwrap();
                update_tx.send(true).unwrap();

                // trigger relayout when new view is available
                sink.send(Box::new(|_: &mut Cursive| {}))
            })
            .unwrap();

        let update_sink = siv.cb_sink().clone();
        // view update thread targeting 30fps
        thread::Builder::new()
            .name(format!("cursive-async-view::updater"))
            .spawn(move || {
                loop {
                    if update_rx.recv_timeout(Duration::from_millis(33)).is_ok() {
                        // flippity flop, I need to stop
                        break;
                    }

                    update_sink.send(Box::new(|_: &mut Cursive| {})).unwrap();
                }
            })
            .unwrap();

        Self {
            view: None,
            loading: TextView::new(""),
            animation_fn: Box::new(default_animation),
            width: None,
            height: None,
            pos: 0,
            rx,
        }
    }

    pub fn with_width(self, width: usize) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn with_height(self, height: usize) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    pub fn with_animation_fn<F>(self, animation_fn: F) -> Self
    where
    // We cannot use a lifetime bound to the AsyncView struct because View has a
    //  'static requirement. Therefore we have to make sure the animation_fn is
    // 'static, meaning it owns all values and does not reference anything
    // outside of its scope. In practice this means all animation_fn must be
    // `move |width| {...}` or fn's.
        F: Fn(usize, usize, usize) -> AnimationFrame + 'static,
    {
        Self {
            animation_fn: Box::new(animation_fn),
            ..self
        }
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = Some(width);
    }

    pub fn inherit_width(&mut self) {
        self.width = None;
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = Some(height);
    }

    pub fn inherit_height(&mut self) {
        self.height = None;
    }

    pub fn set_animation_fn<F>(&mut self, animation_fn: F)
    where
        F: Fn(usize, usize, usize) -> AnimationFrame + 'static
    {
        self.animation_fn = Box::new(animation_fn);
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
        match self.view {
            Some(ref mut view) => view.layout(vec),
            None => self.loading.layout(vec),
        }
    }

    fn needs_relayout(&self) -> bool {
        match self.view {
            Some(ref view) => view.needs_relayout(),
            None => true,
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
            None => {
                let width = self.width.unwrap_or(constraint.x);
                let height = self.height.unwrap_or(constraint.y);

                let AnimationFrame {
                    content,
                    next_frame_idx,
                } = (self.animation_fn)(width, height, self.pos);
                self.loading.set_content(content);
                self.pos = next_frame_idx;

                self.loading.required_size(constraint)
            },
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
