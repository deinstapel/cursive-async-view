use crossbeam::channel::{bounded, unbounded, Receiver, Sender};
use cursive::direction::Direction;
use cursive::event::{AnyCb, Event, EventResult};
use cursive::theme::PaletteColor;
use cursive::utils::markup::StyledString;
use cursive::view::{Selector, View};
use cursive::views::TextView;
use cursive::{Cursive, Printer, Rect, Vec2};
use interpolation::Ease;
use log::warn;
use num::clamp;
use send_wrapper::SendWrapper;

use std::thread;
use std::time::Instant;

use crate::{infinite::FPS, utils, AsyncView};

/// An enum to be returned by the `poll_ready` callback, with additional information about the creation progress.
pub enum AsyncProgressState<V: View> {
    /// Indicates a not completed creation, which is still ongoing. Also reports the progress made as float value between 0 and 1.
    Pending(f32),
    /// Indicates a not completed creation, which cannot proceed further. Contains an error message to be displayed for the user.
    Error(String),
    /// Indicates a completed creation. Contains the new child view.
    Available(V),
}

/// This struct contains the content of a single frame for `AsyncProgressView` with some metadata about the current frame.
pub struct AnimationProgressFrame {
    /// Stylized String which gets printed until the view is ready, or if the creation has failed.
    pub content: StyledString,
    /// Current position of the loading bar.
    pub pos: usize,
    /// Index of the next frame to be drawn, useful if you want to interpolate between two states of progress.
    pub next_frame_idx: usize,
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
/// use cursive_async_view::{AnimationProgressFrame, AsyncProgressView, AsyncProgressState};
///
/// fn my_progress_function(
///     _width: usize,
///     _height: usize,
///     progress: f32,
///     _pos: usize,
///     frame_idx: usize,
/// ) -> AnimationProgressFrame {
///     AnimationProgressFrame {
///         content: StyledString::plain(format!("{:.0}%", progress * 100.0)),
///         pos: 0,
///         next_frame_idx: frame_idx,
///     }
/// }
///
/// let mut siv = Cursive::default();
/// let start = std::time::Instant::now();
/// let async_view = AsyncProgressView::new(&mut siv, move || {
///     if start.elapsed().as_secs() > 5 {
///         AsyncProgressState::Pending(start.elapsed().as_secs() as f32 /5f32)
///     } else {
///         AsyncProgressState::Available(TextView::new("Loaded!"))
///     }
/// })
/// .with_progress_fn(my_progress_function);
/// ```
///
/// The progress function will display the progress in percent as a simple string.
///
/// The `width` and `height` parameters contain the maximum size the content may have
/// (in characters). The `progress` parameter is guaranteed to be a `f32` between 0 and 1.
/// The `pos` and `frame_idx` parameter are always from the animation frame of the previous iteration.
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

    let duration = 30; //one second
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

/// The default error animation for a `AsyncProgressView`.
///
/// # Creating your own error animation
///
/// The creation is very similar to the progress animation, but the error message is given now as the first parameter.
///
/// ```
/// use crossbeam::Sender;
/// use cursive::Cursive;
/// use cursive::views::TextView;
/// use cursive::utils::markup::StyledString;
/// use cursive_async_view::{AnimationProgressFrame, AsyncProgressView, AsyncProgressState};
///
/// fn my_error_function(
///     msg: String,
///     _width: usize,
///     _height: usize,
///     progress: f32,
///     _pos: usize,
///     frame_idx: usize,
/// ) -> AnimationProgressFrame {
///     AnimationProgressFrame {
///         content: StyledString::plain(format!("Error: {}", msg)),
///         pos: 0,
///         next_frame_idx: frame_idx,
///     }
/// }
///
/// let mut siv = Cursive::default();
/// let start = std::time::Instant::now();
/// let async_view = AsyncProgressView::new(&mut siv, move || {
///     if start.elapsed().as_secs() > 5 {
///         AsyncProgressState::Pending(start.elapsed().as_secs() as f32 /5f32)
///     } else if true {
///         AsyncProgressState::Error("Oh no, the view could not be loaded!".to_string())
///     } else {
///         AsyncProgressState::Available(TextView::new("I thought we never would get here!"))
///     }
/// })
/// .with_error_fn(my_error_function);
/// ```
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
        offset += 2_f64;
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
        pos,
        next_frame_idx: frame_idx + 1,
    }
}

/// An `AsyncProgressView` is a wrapper view that displays a progress bar, until the
/// child view is successfully created or an error in the creation progress occured.
///
/// To achieve this a `poll_ready` callback is passed in the creation of `AsyncProgressView` which
/// returns an `AsyncProgressState` that can indicate that the process is still `Pending` (this contains a float
/// between 0 and 1, communicating the progress, this information is displayed in the bar), has been successfully
/// completed `Available` containing the view to be displayed, or if the creation has thrown an `Error`
/// containing a message to be shown to the user.
///
/// The `poll_ready` callback should only **check** for data to be
/// available and create the child view when the data got available. It must
/// **never** block until the data is available or do heavy calculations!
/// Otherwise cursive cannot proceed displaying and your
/// application will have a blocking loading process!
///
/// If you have troubles and need some more in-depth examples have a look at the provided `examples` in the project.
///
/// # Example usage
///
/// ```
/// use cursive::{views::TextView, Cursive};
/// use cursive_async_view::{AsyncProgressView, AsyncProgressState};
///
/// let mut siv = Cursive::default();
/// let start = std::time::Instant::now();
/// let async_view = AsyncProgressView::new(&mut siv, move || {
///     if start.elapsed().as_secs() < 3 {
///         AsyncProgressState::Pending(start.elapsed().as_secs() as f32 / 3f32)
///     } else {
///         AsyncProgressState::Available(TextView::new("Finally it loaded!"))
///     }
/// });
///
/// siv.add_layer(async_view);
/// // siv.run();
/// ```
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
    dropped: Sender<()>,
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
        let (error_tx, error_rx) = bounded(1);

        Self::polling_cb(
            siv,
            Instant::now(),
            SendWrapper::new(view_tx),
            error_rx,
            creator,
        );

        Self {
            view: AsyncProgressState::Pending(0.0),
            loading: TextView::new(""),
            progress_fn: Box::new(default_progress),
            error_fn: Box::new(default_progress_error),
            width: None,
            height: None,
            view_rx,
            frame_index: 0,
            dropped: error_tx,
            pos: 0,
        }
    }

    fn polling_cb<F>(
        siv: &mut Cursive,
        instant: Instant,
        chan: SendWrapper<Sender<AsyncProgressState<T>>>,
        error_chan: Receiver<()>,
        mut cb: F,
    ) where
        F: FnMut() -> AsyncProgressState<T> + 'static,
    {
        let res = cb();
        match res {
            AsyncProgressState::Pending(_) => {
                let sink = siv.cb_sink().clone();
                let cb = SendWrapper::new(cb);
                match chan.send(res) {
                    Ok(_) => {},
                    Err(send_err) => warn!("Could not send progress to AsyncProgressView. It probably has been dropped before the asynchronous initialization of a view has been finished: {}", send_err),
                }
                thread::spawn(move || {
                    // ensure ~60fps
                    if let Some(duration) = FPS.checked_sub(instant.elapsed()) {
                        thread::sleep(duration);
                    }

                    match sink.send(Box::new(move |siv| {
                        Self::polling_cb(siv, Instant::now(), chan, error_chan, cb.take())
                    })) {
                        Ok(_) => {}
                        Err(send_err) => {
                            warn!("Could not send callback to cursive. It probably has been dropped before the asynchronous initialization of a view has been finished: {}", send_err);
                        }
                    }
                });
            }
            AsyncProgressState::Error(content) => {
                AsyncView::<T>::error_anim_cb(siv, error_chan);

                match chan.send(AsyncProgressState::Error(content)) {
                    Ok(_) => {}
                    Err(send_err) => {
                        warn!("View has been dropped before asynchronous initialization has been finished. Check if you removed this view from Cursive: {}", send_err);
                    }
                }
                // chan dropped here, so the rx must handle disconnected
            }
            AsyncProgressState::Available(view) => {
                match chan.send(AsyncProgressState::Available(view)) {
                    Ok(_) => {}
                    Err(send_err) => {
                        warn!("View has been dropped before asynchronous initialization has been finished. Check if you removed this view from Cursive: {}", send_err);
                    }
                }
            }
        }
    }

    /// Mark the maximum allowed width in characters, the progress bar may consume.
    /// By default, the width will be inherited by the parent view.
    pub fn with_width(mut self, width: usize) -> Self {
        self.set_width(width);
        self
    }

    /// Mark the maximum allowed height in characters, the progress bar may consume.
    /// By default, the height will be inherited by the parent view.
    pub fn with_height(mut self, height: usize) -> Self {
        self.set_height(height);
        self
    }

    /// Set a custom progress function for this view, indicating the progress of the
    /// wrapped view creation. See the `default_progress` function reference for an
    /// example on how to create a custom progress function.
    pub fn with_progress_fn<F>(mut self, progress_fn: F) -> Self
    where
        F: Fn(usize, usize, f32, usize, usize) -> AnimationProgressFrame + 'static,
    {
        self.set_progress_fn(progress_fn);
        self
    }

    pub fn with_error_fn<F>(mut self, error_fn: F) -> Self
    where
        F: Fn(String, usize, usize, f32, usize, usize) -> AnimationProgressFrame + 'static,
    {
        self.set_error_fn(error_fn);
        self
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

    /// Set a custom error function for this view, indicating that an error occured during the
    /// wrapped view creation. See the `default_progress_error` function reference for an
    /// example on how to create a custom error function.
    ///
    /// The function may be set at any time. The progress bar can be changed even if
    /// the previous progress bar has already be drawn.
    pub fn set_error_fn<F>(&mut self, error_fn: F)
    where
        F: Fn(String, usize, usize, f32, usize, usize) -> AnimationProgressFrame + 'static,
    {
        self.error_fn = Box::new(error_fn);
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

impl<T: View> Drop for AsyncProgressView<T> {
    fn drop(&mut self) {
        match self.dropped.send(()) {
            Ok(_) => {}
            Err(send_err) => warn!(
                "Refreshing thread has been dropped before view has, this has no impact on your code and is a bug: {}",
                send_err
            ),
        }
    }
}

impl<T: View + Sized> View for AsyncProgressView<T> {
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
        if match self.view {
            AsyncProgressState::Available(_) => false,
            _ => true,
        } {
            if let Ok(state) = self.view_rx.try_recv() {
                self.view = state
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
                    (*msg).to_string(),
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
