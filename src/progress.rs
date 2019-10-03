use crossbeam::channel::{unbounded, Receiver, Sender};
use cursive::direction::Direction;
use cursive::event::{AnyCb, Event, EventResult};
use cursive::theme::PaletteColor;
use cursive::utils::markup::StyledString;
use cursive::view::{Selector, View};
use cursive::views::TextView;
use cursive::{Cursive, Printer, Rect, Vec2};
use interpolation::Ease;
use num::clamp;
use send_wrapper::SendWrapper;

use std::thread;
use std::time::{Duration, Instant};

use crate::{infinite::FPS, utils};

pub enum AsyncProgressState<V: View> {
    Pending(f32),
    Error(String),
    Available(V),
}

pub struct AnimationProgressFrame {
    content: StyledString,
    pos: usize,
    next_frame_idx: usize,
}

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
pub fn default_progress(
    width: usize,
    _height: usize,
    progress: f32,
    pos: usize,
    frame_idx: usize,
) -> AnimationProgressFrame {
    assert!(progress >= 0.0);
    assert!(progress <= 1.0);

    let foreground = PaletteColor::Highlight;
    let background = PaletteColor::HighlightInactive;
    let symbol = "━";

    let duration = 30; //half a second
    let durationf = duration as f64;

    let next_pos = width as f32 * progress;
    let offset = next_pos as usize - pos;

    let idx = frame_idx % duration;
    let idxf = idx as f64;
    let factor = (idxf / durationf).circular_out();
    let end = (pos as f64 + offset as f64 * factor) as usize;

    let mut result = StyledString::new();
    result.append_styled(utils::repeat_str(symbol, end), foreground);
    result.append_styled(utils::repeat_str(symbol, width - end), background);

    AnimationProgressFrame {
        content: result,
        pos: end,
        next_frame_idx: idx + 1,
    }
}

pub fn default_progress_error(
    msg: String,
    width: usize,
    _height: usize,
    progress: f32,
    pos: usize,
    frame_idx: usize,
) -> AnimationProgressFrame {
    assert!(progress >= 0.0);
    assert!(progress <= 1.0);

    let foreground = PaletteColor::Highlight;
    let background = PaletteColor::HighlightInactive;
    let symbol = "━";

    let duration = 30; // half a second
    let durationf = duration as f64;
    let idx = frame_idx;
    let idxf = idx as f64;
    let factor = (idxf / durationf).circular_in_out();
    let mut offset = width as f64 * factor;

    let padding = width.saturating_sub(msg.len()) / 2;
    let mut background_content = format!(
        "{}{}{}",
        utils::repeat_str(" ", padding),
        msg,
        utils::repeat_str(" ", padding),
    );
    // Check for non-char symbols
    if background_content
        .as_str()
        .get(0..offset as usize)
        .is_none()
    {
        offset = offset + 2 as f64;
    }
    let end = pos + offset as usize;
    background_content.truncate(offset as usize);
    let mut result = StyledString::new();
    result.append_plain(background_content);
    result.append_styled(
        utils::repeat_str(symbol, {
            if (pos + offset as usize) < width {
                pos
            } else {
                width.saturating_sub(offset as usize)
            }
        }),
        foreground,
    );
    result.append_styled(
        utils::repeat_str(symbol, width.saturating_sub(end)),
        background,
    );

    AnimationProgressFrame {
        content: result,
        pos: pos,
        next_frame_idx: frame_idx + 1,
    }
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
///r
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
pub struct AsyncProgressView<T: View> {
    view: AsyncProgressState<T>,
    loading: TextView,
    progress_fn: Box<dyn Fn(usize, usize, f32, usize, usize) -> AnimationProgressFrame + 'static>,
    error_fn:
        Box<dyn Fn(String, usize, usize, f32, usize, usize) -> AnimationProgressFrame + 'static>,
    width: Option<usize>,
    height: Option<usize>,
    view_rx: Receiver<AsyncProgressState<T>>,
    frame_index: usize,
    pos: usize,
}

impl<T: View> AsyncProgressView<T> {
    /// Create a new `AsyncProgressView` instance. The cursive reference is only used to
    /// update the screen when a progress update is received. In order to show the view,
    /// it has to be directly or indirectly added to a cursive layer like any other view.
    ///
    /// The creator function will be executed on a dedicated thread in the background.
    /// Make sure that this function will never block indefinitely. Otherwise, the
    /// creation thread will get stuck.
    pub fn new<F>(siv: &mut Cursive, creator: F) -> Self
    where
        F: FnMut() -> AsyncProgressState<T> + 'static,
    {
        let (view_tx, view_rx) = unbounded();

        Self::polling_cb(siv, Instant::now(), SendWrapper::new(view_tx), creator);

        Self {
            view: AsyncProgressState::Pending(0.0),
            loading: TextView::new(""),
            progress_fn: Box::new(default_progress),
            error_fn: Box::new(default_progress_error),
            width: None,
            height: None,
            view_rx: view_rx,
            frame_index: 0,
            pos: 0,
        }
    }

    fn polling_cb<F>(
        siv: &mut Cursive,
        instant: Instant,
        chan: SendWrapper<Sender<AsyncProgressState<T>>>,
        mut cb: F,
    ) where
        F: FnMut() -> AsyncProgressState<T> + 'static,
    {
        let res = cb();
        match res {
            AsyncProgressState::Pending(_) => {
                let sink = siv.cb_sink().clone();
                let cb = SendWrapper::new(cb);
                // Progress send
                chan.send(res).unwrap();
                thread::spawn(move || {
                    // ensure ~60fps
                    if let Some(duration) = FPS.checked_sub(instant.elapsed()) {
                        thread::sleep(duration);
                    }

                    sink.send(Box::new(move |siv| {
                        Self::polling_cb(siv, Instant::now(), chan, cb.take())
                    }))
                    .unwrap();
                });
            }
            state => {
                // For now workaround
                let sink = siv.cb_sink().clone();
                thread::spawn(move || loop {
                    thread::sleep(Duration::from_millis(16));
                    sink.send(Box::new(|_| {})).unwrap();
                });
                chan.send(state).unwrap();
                // chan dropped here, so the rx must handle disconnected
            }
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
        F: Fn(usize, usize, f32, usize, usize) -> AnimationProgressFrame + 'static,
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
        F: Fn(usize, usize, f32, usize, usize) -> AnimationProgressFrame + 'static,
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
        match &self.view {
            AsyncProgressState::Available(v) => {
                v.draw(printer);
            }
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.draw(printer)
            }
        }
    }

    fn layout(&mut self, vec: Vec2) {
        match &mut self.view {
            AsyncProgressState::Available(v) => v.layout(vec),
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.layout(vec)
            }
        }
    }

    fn needs_relayout(&self) -> bool {
        match &self.view {
            AsyncProgressState::Available(v) => v.needs_relayout(),
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.needs_relayout()
            }
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        if {
            match self.view {
                AsyncProgressState::Available(_) => false,
                _ => true,
            }
        } {
            match self.view_rx.try_recv() {
                Ok(state) => self.view = state,
                Err(_) => {}
            }
        }

        match &mut self.view {
            AsyncProgressState::Available(v) => v.required_size(constraint),
            AsyncProgressState::Pending(value) => {
                let width = self.width.unwrap_or(constraint.x);
                let height = self.height.unwrap_or(constraint.y);
                let AnimationProgressFrame {
                    content,
                    pos,
                    next_frame_idx,
                } = (self.progress_fn)(
                    width,
                    height,
                    clamp(*value, 0.0, 1.0),
                    self.pos,
                    self.frame_index,
                );
                self.pos = pos;
                self.frame_index = next_frame_idx;
                self.loading.set_content(content);
                self.loading.required_size(constraint)
            }
            AsyncProgressState::Error(msg) => {
                let width = self.width.unwrap_or(constraint.x);
                let height = self.height.unwrap_or(constraint.y);
                let AnimationProgressFrame {
                    content,
                    pos,
                    next_frame_idx,
                } = (self.error_fn)(
                    msg.to_string(),
                    width,
                    height,
                    0.5,
                    self.pos,
                    self.frame_index,
                );
                self.pos = pos;
                self.frame_index = next_frame_idx;
                self.loading.set_content(content);
                self.loading.required_size(constraint)
            }
        }
    }

    fn on_event(&mut self, ev: Event) -> EventResult {
        match &mut self.view {
            AsyncProgressState::Available(v) => v.on_event(ev),
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.on_event(ev)
            }
        }
    }

    fn call_on_any<'a>(&mut self, sel: &Selector, cb: AnyCb<'a>) {
        match &mut self.view {
            AsyncProgressState::Available(v) => v.call_on_any(sel, cb),
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.call_on_any(sel, cb)
            }
        }
    }

    fn focus_view(&mut self, sel: &Selector) -> Result<(), ()> {
        match &mut self.view {
            AsyncProgressState::Available(v) => v.focus_view(sel),
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.focus_view(sel)
            }
        }
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match &mut self.view {
            AsyncProgressState::Available(v) => v.take_focus(source),
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.take_focus(source)
            }
        }
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        match &self.view {
            AsyncProgressState::Available(v) => v.important_area(view_size),
            AsyncProgressState::Error(_) | AsyncProgressState::Pending(_) => {
                self.loading.important_area(view_size)
            }
        }
    }
}
