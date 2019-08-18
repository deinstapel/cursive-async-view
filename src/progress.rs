use std::thread;

use cursive::{Cursive, Printer, Rect, Vec2};
use cursive::event::{Event, EventResult, AnyCb};
use cursive::direction::Direction;
use cursive::view::{View, Selector};
use cursive::views::{TextView};
use cursive::utils::markup::StyledString;
use cursive::theme::PaletteColor;
use crossbeam::channel::{self, Sender, Receiver};
use num::clamp;

use crate::utils;

pub fn default_progress(width: usize, _height: usize, progress: f32) -> StyledString {
    assert!(progress >= 0.0);
    assert!(progress <= 1.0);

    let foreground = PaletteColor::Highlight;
    let background = PaletteColor::HighlightInactive;
    let symbol = "━";

    let end = (progress * width as f32) as usize;
    let mut result = StyledString::new();
    result.append_styled(utils::repeat_str(symbol, end), foreground);
    result.append_styled(utils::repeat_str(symbol, width - end), background);

    result
}

pub struct AsyncProgressView<T: View + Send> {
    view: Option<T>,
    loading: TextView,
    progress_fn: Box<dyn Fn(usize, usize, f32) -> StyledString + 'static>,
    width: Option<usize>,
    height: Option<usize>,
    view_rx: Receiver<T>,
    update_rx: Receiver<f32>,
}

impl <T: View + Send + Sized> AsyncProgressView<T> {
    pub fn new<F>(siv: &Cursive, creator: F) -> Self
    where
        F: FnOnce(Sender<f32>) -> T + Send + 'static
    {
        let (view_tx, view_rx) = channel::unbounded();
        let (progress_tx, progress_rx) = channel::unbounded();
        let (update_tx, update_rx) = channel::unbounded();
        let sink = siv.cb_sink().clone();

        thread::Builder::new()
            .name(format!("cursive-async-view::creator"))
            .spawn(move || {
                progress_tx.send(0.0).unwrap();
                view_tx.send(creator(progress_tx)).unwrap();

                // update the layout when the new view is available
                sink.send(Box::new(|_: &mut Cursive| {})).ok();
            })
            .unwrap();

        let update_sink = siv.cb_sink().clone();
        thread::Builder::new()
            .name(format!("cursive-async-view::updater"))
            .spawn(move || {
                loop {
                    // it's okay to make this blocking as we are in a dedicated thread
                    match progress_rx.recv() {
                        Ok(value) => {
                            update_tx.send(value).unwrap();
                            update_sink.send(Box::new(|_: &mut Cursive| {})).ok();
                        },
                        Err(_) => break,
                    }
                }
            })
            .unwrap();

        Self {
            view: None,
            loading: TextView::new(""),
            progress_fn: Box::new(default_progress),
            width: None,
            height: None,
            view_rx,
            update_rx,
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

    pub fn with_progress_fn<F>(self, progress_fn: F) -> Self
    where
        F: Fn(usize, usize, f32) -> StyledString + 'static,
    {
        Self {
            progress_fn: Box::new(progress_fn),
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

    pub fn set_progress_fn<F>(&mut self, progress_fn: F)
    where
        F: Fn(usize, usize, f32) -> StyledString + 'static
    {
        self.progress_fn = Box::new(progress_fn);
    }
}

impl <T: View + Send + Sized> View for AsyncProgressView<T> {
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
            match self.view_rx.try_recv() {
                Ok(view) => self.view = Some(view),
                Err(_) => {},
            }
        }

        match self.view {
            Some(ref mut view) => view.required_size(constraint),
            None => {
                if let Some(value) = self.update_rx.try_recv().ok() {
                    let width = self.width.unwrap_or(constraint.x);
                    let height = self.height.unwrap_or(constraint.y);
                    let content = (self.progress_fn)(width, height, clamp(value, 0.0, 1.0));
                    self.loading.set_content(content);
                }

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
