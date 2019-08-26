use std::thread;

use crossbeam::channel::{self, Receiver, Sender};
use cursive::direction::Direction;
use cursive::event::{AnyCb, Event, EventResult};
use cursive::theme::PaletteColor;
use cursive::utils::markup::StyledString;
use cursive::view::{Selector, View};
use cursive::views::TextView;
use cursive::{Cursive, Printer, Rect, Vec2};
use num::clamp;
use bounded_spsc_queue::{make, Consumer};

use crate::utils;

/// The default progress animation for a `AsyncProgressView`.
///
/// # Creating your own progress function
///
/// As an example a very basic progress function would look like this:
///
/// ```
/// use crossbeam::Sender;
/// use cursive::Cursive;
/// use cursive::views::TextView;
/// use cursive::utils::markup::StyledString;
/// use cursive_async_view::AsyncProgressView;
///
/// fn my_progress_function(
///     _width: usize,
///     _height: usize,
///     progress: f32,
/// ) -> StyledString {
///     StyledString::plain(format!("{:.0}%", progress * 100.0))
/// }
///
/// let mut siv = Cursive::default();
/// let async_view = AsyncProgressView::new(&siv, |s: Sender<f32>| {
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.2).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.4).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.6).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.8).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(1.0).unwrap();
///     TextView::new("Yay, the content has loaded!")
/// })
/// .with_progress_fn(my_progress_function);
/// ```
///
/// The progress function will display the progress in percent as a simple string.
///
/// The `width` and `height` parameters contain the maximum size the content may have
/// (in characters). The `progress` parameter is guaranteed to be a `f32` between 0 and 1.
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

/// An `AsyncProgressView` is a wrapper view that displays a progress bar, until the
/// child view is successfully created. The creation of the inner view is done on a
/// dedicated thread. Therefore, it is necessary for the creation function to always
/// return, otherwise the thread will get stuck.
///
/// # Example usage
///
/// ```
/// use crossbeam::Sender;
/// use cursive::{views::TextView, Cursive};
/// use cursive_async_view::AsyncProgressView;
///
/// let mut siv = Cursive::default();
/// let async_view = AsyncProgressView::new(&siv, |s: Sender<f32>| {
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.2).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.4).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.6).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(0.8).unwrap();
///     std::thread::sleep(std::time::Duration::from_secs(1));
///     s.send(1.0).unwrap();
///     TextView::new("Yay, the content has loaded!")
/// });
///
/// siv.add_layer(async_view);
/// // siv.run();
/// ```
///
/// # Threads
///
/// The `new(siv, creator)` method will spawn 2 threads:
///
/// 1. `cursive-async-view::creator` The creation thread for the wrapped view.
///    This thread will stop running as soon as the creation function returned.
/// 2. `cursive-async-view::updater` The update thread waits for the creation
///    function to signal progress and will be stopped by `AsyncProgressView`
///    when the creation function returned and the new view is available for
///    layouting.
///
/// The threads are labeled as indicated above.
///
/// # TODO
///
/// * make creation function return a result to mark an unsuccessful creation
///
pub struct AsyncProgressView<T: View + Send> {
    view: Option<T>,
    loading: TextView,
    progress_fn: Box<dyn Fn(usize, usize, f32) -> StyledString + 'static>,
    width: Option<usize>,
    height: Option<usize>,
    view_rx: Consumer<T>,
    update_rx: Receiver<f32>,
}

impl<T: View + Send + Sized> AsyncProgressView<T> {
    /// Create a new `AsyncProgressView` instance. The cursive reference is only used to
    /// update the screen when a progress update is received. In order to show the view,
    /// it has to be directly or indirectly added to a cursive layer like any other view.
    ///
    /// The creator function will be executed on a dedicated thread in the background.
    /// Make sure that this function will never block indefinitely. Otherwise, the
    /// creation thread will get stuck.
    pub fn new<F>(siv: &Cursive, creator: F) -> Self
    where
        F: FnOnce(Sender<f32>) -> T + Send + 'static,
    {
        let (progress_tx, progress_rx) = channel::unbounded();
        let (update_tx, update_rx) = channel::unbounded();
        // We use this channel exactly once, so we can use a bounded one
        let (view_tx, view_rx) = make(1);
        let sink = siv.cb_sink().clone();

        thread::Builder::new()
            .name(format!("cursive-async-view::creator"))
            .spawn(move || {
                progress_tx.send(0.0).unwrap();
                view_tx.push(creator(progress_tx));

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
                        }
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

    /// Mark the maximum allowed width in characters, the progress bar may consume.
    /// By default, the width will be inherited by the parent view.
    pub fn with_width(self, width: usize) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    /// Mark the maximum allowed height in characters, the progress bar may consume.
    /// By default, the height will be inherited by the parent view.
    pub fn with_height(self, height: usize) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    /// Set a custom progress function for this view, indicating the progress of the
    /// wrapped view creation. See the `default_progress` function reference for an
    /// example on how to create a custom progress function.
    pub fn with_progress_fn<F>(self, progress_fn: F) -> Self
    where
        F: Fn(usize, usize, f32) -> StyledString + 'static,
    {
        Self {
            progress_fn: Box::new(progress_fn),
            ..self
        }
    }

    /// Set the maximum allowed width in characters, the progress bar may consume.
    pub fn set_width(&mut self, width: usize) {
        self.width = Some(width);
    }

    /// Set the maximum allowed height in characters, the progress bar may consume.
    pub fn set_height(&mut self, height: usize) {
        self.height = Some(height);
    }

    /// Set a custom progress function for this view, indicating the progress of the
    /// wrapped view creation. See the `default_progress` function reference for an
    /// example on how to create a custom progress function.
    ///
    /// The function may be set at any time. The progress bar can be changed even if
    /// the previous progress bar has already be drawn.
    pub fn set_progress_fn<F>(&mut self, progress_fn: F)
    where
        F: Fn(usize, usize, f32) -> StyledString + 'static,
    {
        self.progress_fn = Box::new(progress_fn);
    }

    /// Make the progress bar inherit its width from the parent view. This is the default.
    pub fn inherit_width(&mut self) {
        self.width = None;
    }

    /// Make the progress bar inherit its height from the parent view. This is the default.
    pub fn inherit_height(&mut self) {
        self.height = None;
    }
}

impl<T: View + Send + Sized> View for AsyncProgressView<T> {
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
            match self.view_rx.try_pop() {
                Some(view) => self.view = Some(view),
                None => {}
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
            }
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
