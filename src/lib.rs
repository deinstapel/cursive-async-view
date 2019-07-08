use std::sync::mpsc::{self, Receiver};
use cursive::{wrap_impl, Cursive, Printer, Rect, Vec2};
use cursive::event::{Event, EventResult, AnyCb};
use cursive::direction::Direction;
use cursive::view::{View, ViewWrapper, Selector};
use cursive::views::{TextView};

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
            loading: TextView::new("Loading..."),
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
