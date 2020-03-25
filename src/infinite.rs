use std::thread;
use std::time::{Duration, Instant};

use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use cursive::align::HAlign;
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

use crate::utils;

/// This struct represents the content of a single loading or error animation frame,
/// produced by a animation function of the `AsyncView`. Read the documentation
/// of the `default_animation` or `default_error` to see how to implement your own
/// animation functions.
pub struct AnimationFrame {
    /// A `StyledString` that will be displayed inside a `TextView` for this frame.
    pub content: StyledString,

    /// The next `frame_idx` passed to the animation function when calculating
    /// the next frame.
    pub next_frame_idx: usize,
}

/// The default loading animation for a `AsyncView`.
///
/// # Creating your own loading function
///
/// As an example a very basic loading function would look like this:
///
/// ```
/// use std::time::{Instant, Duration};
/// use cursive::Cursive;
/// use cursive::views::TextView;
/// use cursive::utils::markup::StyledString;
/// use cursive_async_view::{AsyncView, AsyncState, AnimationFrame};
///
/// fn my_loading_animation(
///     _width: usize,
///     _height: usize,
///     frame_idx: usize,
/// ) -> AnimationFrame {
///     let content = if frame_idx < 30 {
///         StyledString::plain("loading")
///     } else {
///         StyledString::plain("content")
///     };
///
///     AnimationFrame {
///         content,
///         next_frame_idx: (frame_idx + 1) % 60,
///     }
/// }
///
/// let mut siv = Cursive::default();
/// let instant = Instant::now();
/// let async_view = AsyncView::new(&mut siv, move || {
///     if instant.elapsed() > Duration::from_secs(5) {
///         AsyncState::Available(
///             TextView::new("Yay!\n\nThe content has loaded!")
///         )
///     } else {
///         AsyncState::Pending
///     }
/// }).with_animation_fn(my_loading_animation);
///
/// siv.add_layer(async_view);
/// // siv.run();
/// ```
///
/// This animation function will first display `loading` for half a second and then display
/// `content` for half a second.
///
/// The `width` and `height` parameters contain the maximum size the content may have
/// (in characters). The initial `frame_idx` is 0.
pub fn default_animation(width: usize, _height: usize, frame_idx: usize) -> AnimationFrame {
    let foreground = PaletteColor::Highlight;
    let background = PaletteColor::HighlightInactive;
    let symbol = "━";

    let duration = 60; // one second
    let durationf = duration as f64;

    let idx = frame_idx % duration;
    let idxf = idx as f64;
    let factor = idxf / durationf;
    let begin_factor = clamp((factor % 1.0).circular_in_out(), 0.0, 1.0);
    let end_factor = clamp(((factor + 0.25) % 1.0).circular_in_out() * 2.0, 0.0, 1.0);
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

/// The default error animation for a `AsyncView`.
///
/// # Creating your own error function
///
/// As an example a very basic error function would look like this:
///
/// ```
/// use std::time::{Instant, Duration};
/// use cursive::Cursive;
/// use cursive::views::TextView;
/// use cursive::utils::markup::StyledString;
/// use cursive_async_view::{AsyncView, AsyncState, AnimationFrame};
///
/// fn my_error_animation(
///     msg: &str,
///     _width: usize,
///     _height: usize,
///     _error_idx: usize,
///     _frame_idx: usize,
/// ) -> AnimationFrame {
///     AnimationFrame {
///         content: StyledString::plain(msg),
///         next_frame_idx: 0,
///     }
/// }
///
/// let mut siv = Cursive::default();
/// let instant = Instant::now();
/// let async_view: AsyncView<TextView> = AsyncView::new(&mut siv, move || {
///     if instant.elapsed() > Duration::from_secs(5) {
///         AsyncState::Error("Oh no, an error occured!".to_string())
///     } else {
///         AsyncState::Pending
///     }
/// }).with_error_fn(my_error_animation);
///
/// siv.add_layer(async_view);
/// // siv.run();
/// ```
///
/// This error function will just display the error message itself.
///
/// The `width` and `height` prameters contain the maximum size the content may have
/// (in characters). The initial `frame_idx` is 0.
pub fn default_error(
    msg: &str,
    width: usize,
    _height: usize,
    error_idx: usize,
    frame_idx: usize,
) -> AnimationFrame {
    let foreground = PaletteColor::Highlight;
    let background = PaletteColor::HighlightInactive;
    let symbol = "━";

    let offset = utils::repeat_str(" ", HAlign::Center.get_offset(msg.len(), width));
    let mut msg = format!("{}{}{}", offset, msg, offset);

    let duration = 60; // one second
    let durationf = duration as f64;
    let cycle = if error_idx % duration > duration / 2 {
        duration
    } else {
        0
    };

    let idx = frame_idx - (error_idx / duration) * duration;
    let idxf = idx as f64;
    let factor = idxf / durationf;
    let begin_factor = clamp((factor % 1.0).circular_in_out(), 0.0, 1.0);
    let end_factor = clamp(((factor + 0.25) % 1.0).circular_in_out() * 2.0, 0.0, 1.0);
    let mut begin = (begin_factor * width as f64) as usize;
    let end = (end_factor * width as f64) as usize;
    if frame_idx == cycle + duration {
        // Text can be fully shown
        return AnimationFrame {
            content: StyledString::plain(msg),
            next_frame_idx: frame_idx,
        };
    }

    let mut result = StyledString::default();
    if end >= begin && idx > cycle {
        if msg.as_str().get(0..begin).is_none() {
            begin += 2;
        }
        msg.truncate(begin);
        result.append_plain(msg);
        result.append_styled(utils::repeat_str(symbol, end - begin), foreground);
        result.append_styled(utils::repeat_str(symbol, width - end), background);
    } else if end >= begin && idx <= cycle {
        result.append_styled(utils::repeat_str(symbol, begin), background);
        result.append_styled(utils::repeat_str(symbol, end - begin), foreground);
        result.append_styled(utils::repeat_str(symbol, width - end), background);
    } else if idx > cycle + duration / 2 {
        if msg.as_str().get(0..begin).is_none() {
            begin += 2
        }
        msg.truncate(begin);
        result.append_plain(msg);
        result.append_styled(utils::repeat_str(symbol, width - begin), foreground);
    } else {
        // Complete animation until text can be unveiled
        result.append_styled(utils::repeat_str(symbol, end), foreground);
        result.append_styled(utils::repeat_str(symbol, begin - end), background);
        result.append_styled(utils::repeat_str(symbol, width - begin), foreground);
    }

    AnimationFrame {
        content: result,
        next_frame_idx: frame_idx + 1,
    }
}

/// This enum is used in the ready_poll callback to tell the async view
/// whether the view is already available, an error occured, or is still pending.
pub enum AsyncState<V: View> {
    /// The view of type `V` is now available and ready to be owned by the async view
    /// where it will get layouted and drawn instead of the loading animation.
    Available(V),

    /// Loading of the view failed with the given error.
    Error(String),

    /// The view is not available yet, try again later.
    Pending,
}

/// An `AsyncView` is a wrapper view that displays a loading screen, until the
/// child view is ready to be created. The view can be used in two different
/// ways.
///
/// # Poll-based AsyncView
///
/// The poll-based `AsyncView` is constructed via the `AsyncView::new` function
/// and regularly calls the provided `poll_ready` function. It indicates
/// whether the child view is available or not by returning an `AsyncState`
/// enum. The `poll_ready` callback should only **check** for data to be
/// available and create the child view when the data got available. It must
/// **never** block until the data is available or do heavy calculations!
///
/// Use a different thread for long taking calculations. Check the `bg_task`
/// example for an example on how to use a dedicated calculation thread with
/// the `AsyncView`.
///
/// ## Example usage of the poll-based variant
///
/// ```
/// use std::time::{Instant, Duration};
/// use cursive::{views::TextView, Cursive};
/// use cursive_async_view::{AsyncView, AsyncState};
///
/// let mut siv = Cursive::default();
/// let instant = Instant::now();
/// let async_view = AsyncView::new(&mut siv, move || {
///     // check if the view can be created
///     if instant.elapsed() > Duration::from_secs(10) {
///         AsyncState::Available(
///             TextView::new("Yay!\n\nThe content has loaded!")
///         )
///     } else {
///         AsyncState::Pending
///     }
/// });
///
/// siv.add_layer(async_view);
/// // siv.run();
/// ```
///
/// The content will be displayed after 10 seconds.
///
/// # Producing view data in a background thread
///
/// The second variant produces custom data in a background thread via the
/// provided `bg_task` function. The produced data is then sent to the cursive
/// thread and given to the provided `view_creator` function. This function
/// should construct the child view and return it to the async view.
///
/// All heavy work **must** be done in the `bg_task` function. Otherwise,
/// the cursive event loop will be blocked, preventing any rendering or event
/// handling taking place.
///
/// ## Example usage for the background thread variant
///
/// ```
/// use std::thread;
/// use std::time::Duration;
///
/// use cursive::views::TextView;
/// use cursive::Cursive;
/// use cursive_async_view::AsyncView;
///
/// let mut siv = Cursive::default();
/// let async_view = AsyncView::new_with_bg_creator(&mut siv, move || {
///     // this function is executed in a background thread, so we can block
///     // here as long as we like
///     thread::sleep(Duration::from_secs(10));
///
///     // enough blocking, let's show the content
///     Ok("Yeet! It worked 🖖")
/// }, TextView::new); // create a text view from the string
///
/// siv.add_layer(async_view);
/// // siv.run();
/// ```
///
/// The content will be displayed after 10 seconds.
pub struct AsyncView<T: View> {
    view: AsyncState<T>,
    loading: TextView,
    animation_fn: Box<dyn Fn(usize, usize, usize) -> AnimationFrame + 'static>,
    error_fn: Box<dyn Fn(&str, usize, usize, usize, usize) -> AnimationFrame + 'static>,
    width: Option<usize>,
    height: Option<usize>,
    pos: usize,
    error_idx: usize,
    rx: Receiver<AsyncState<T>>,
    error_sender: Sender<()>,
}

lazy_static::lazy_static! {
    pub(crate) static ref FPS: Duration = Duration::from_secs(1) / 60;
}

impl<T: View> AsyncView<T> {
    /// Create a new `AsyncView` instance. The cursive reference is used
    /// to control the refresh rate of the terminal when the loading animation
    /// is running. In order to show the view, it has to be directly or indirectly
    /// added to a cursive layer like any other view.
    ///
    /// The `ready_poll` function will be called regularly until the view has
    /// either been loaded or errored. Use this function only to check whether
    /// your data is available. Do not run heavy calculations in this function.
    /// Instead use a dedicated thread for it as shown in the `bg_task` example.
    pub fn new<F>(siv: &mut Cursive, ready_poll: F) -> Self
    where
        F: FnMut() -> AsyncState<T> + 'static,
    {
        // create communication channel between cursive event loop and
        // this views layout code
        let (tx, rx) = channel::unbounded();
        let (error_tx, error_rx) = channel::bounded(1);

        let instant = Instant::now();
        Self::polling_cb(siv, instant, SendWrapper::new(tx), error_rx, ready_poll);

        Self {
            view: AsyncState::Pending,
            loading: TextView::new(""),
            animation_fn: Box::new(default_animation),
            error_fn: Box::new(default_error),
            width: None,
            height: None,
            pos: 0,
            error_idx: 0,
            rx,
            error_sender: error_tx,
        }
    }

    /// Create a new `AsyncView` instance. The cursive reference is used
    /// to control the refresh rate of the terminal when the loading animation
    /// is running. In order to show the view, it has to be directly or indirectly
    /// added to a cursive layer like any other view.
    ///
    /// The `bg_task` function is executed on a background thread called
    /// `cursive-async-view::bg_task`. It should be used to produce data of
    /// type `D` which is converted to a view by the `view_creator` function.
    pub fn new_with_bg_creator<F, C, D>(siv: &mut Cursive, bg_task: F, mut view_creator: C) -> Self
    where
        D: Send + 'static,
        F: FnOnce() -> Result<D, String> + Send + 'static,
        C: FnMut(D) -> T + 'static,
    {
        let (tx, rx) = channel::unbounded();

        thread::Builder::new()
            .name("cursive-async-view::bg_task".into())
            .spawn(move || {
                tx.send(bg_task()).unwrap();
            })
            .unwrap();

        Self::new(siv, move || match rx.try_recv() {
            Ok(Ok(data)) => AsyncState::Available(view_creator(data)),
            Ok(Err(err)) => AsyncState::Error(err),
            Err(TryRecvError::Empty) => AsyncState::Pending,
            Err(TryRecvError::Disconnected) => {
                AsyncState::Error("Internal error: bg_task disconnected unexpectedly!".to_string())
            }
        })
    }

    fn polling_cb<F>(
        siv: &mut Cursive,
        instant: Instant,
        chan: SendWrapper<Sender<AsyncState<T>>>,
        end_anim: Receiver<()>,
        mut cb: F,
    ) where
        F: FnMut() -> AsyncState<T> + 'static,
    {
        match cb() {
            AsyncState::Pending => {
                let sink = siv.cb_sink().clone();
                let cb = SendWrapper::new(cb);
                thread::spawn(move || {
                    // ensure ~60fps
                    if let Some(duration) = FPS.checked_sub(instant.elapsed()) {
                        thread::sleep(duration);
                    }

                    match sink.send(Box::new(move |siv| {
                        Self::polling_cb(siv, Instant::now(), chan, end_anim, cb.take())
                    })) {
                        Ok(_) => {}
                        Err(send_err) => {
                            warn!("Could not send callback to cursive. It probably has been dropped before the asynchronous initialization of a view has been finished: {}", send_err);
                        }
                    }
                });
            }
            AsyncState::Error(content) => {
                // Start a thread running until the object has been dropped
                Self::error_anim_cb(siv, end_anim);

                // This may panic if the other site has been dropped Can happen
                // if the view gets removed before the event loop has finished
                // causing the sender to try to to communicate with a dead
                // receiver To fix this we drop this error and warn the user
                // that this behaviour is discouraged
                match chan.send(AsyncState::Error(content)) {
                    Ok(_) => {}
                    Err(send_err) => {
                        warn!("View has been dropped before asynchronous initialization has been finished. Check if you removed this view from Cursive: {}", send_err);
                    }
                }
                // chan dropped here, so the rx must handle disconnected
            }
            AsyncState::Available(view) => match chan.send(AsyncState::Available(view)) {
                Ok(_) => {}
                Err(send_err) => {
                    warn!("View has been dropped before asynchronous initialization has been finished. Check if you removed this view from Cursive: {}", send_err);
                }
            },
        }
    }

    pub(crate) fn error_anim_cb(siv: &mut Cursive, chan: Receiver<()>) {
        let sink = siv.cb_sink().clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(16));

            match chan.try_recv() {
                Ok(()) => break,
                Err(_) => match sink.send(Box::new(|_| {})) {
                    Ok(_) => {}
                    Err(send_err) => {
                        warn!(
                            "Cursive has been dropped before AsyncView has been: {}",
                            send_err
                        );
                    }
                },
            }
        });
    }

    /// Mark the maximum allowed width in characters, the loading animation may consume.
    /// By default, the width will be inherited by the parent view.
    pub fn with_width(mut self, width: usize) -> Self {
        self.set_width(width);
        self
    }

    /// Mark the maximum allowed height in characters, the loading animation may consume.
    /// By default, the height will be inherited by the parent view.
    pub fn with_height(mut self, height: usize) -> Self {
        self.set_height(height);
        self
    }

    /// Set a custom animation function for this view, indicating that the wrapped view is
    /// not available yet. See the `default_animation` function reference for an example on
    /// how to create a custom animation function.
    pub fn with_animation_fn<F>(mut self, animation_fn: F) -> Self
    where
        // We cannot use a lifetime bound to the AsyncView struct because View has a
        //  'static requirement. Therefore we have to make sure the animation_fn is
        // 'static, meaning it owns all values and does not reference anything
        // outside of its scope. In practice this means all animation_fn must be
        // `move |width| {...}` or fn's.
        F: Fn(usize, usize, usize) -> AnimationFrame + 'static,
    {
        self.set_animation_fn(animation_fn);
        self
    }

    /// Set a custom error animation function for this view, indicating that the
    /// wrapped view has failed to load. See the `default_error` function
    /// reference for an example on how to create a custom error animation
    /// function.
    pub fn with_error_fn<F>(mut self, error_fn: F) -> Self
    where
        // We cannot use a lifetime bound to the AsyncView struct because View has a
        //  'static requirement. Therefore we have to make sure the error_fn is
        // 'static, meaning it owns all values and does not reference anything
        // outside of its scope. In practice this means all animation_fn must be
        // `move |width| {...}` or fn's.
        F: Fn(&str, usize, usize, usize, usize) -> AnimationFrame + 'static,
    {
        self.set_error_fn(error_fn);
        self
    }

    /// Set the maximum allowed width in characters, the loading animation may consume.
    pub fn set_width(&mut self, width: usize) {
        self.width = Some(width);
    }

    /// Set the maximum allowed height in characters, the loading animation may consume.
    pub fn set_height(&mut self, height: usize) {
        self.height = Some(height);
    }

    /// Set a custom animation function for this view, indicating that the wrapped view is
    /// not available yet. See the `default_animation` function reference for an example on
    /// how to create a custom animation function.
    ///
    /// This function may be set at any time. The loading animation can be changed even if
    /// the previous loading animation has already started.
    pub fn set_animation_fn<F>(&mut self, animation_fn: F)
    where
        F: Fn(usize, usize, usize) -> AnimationFrame + 'static,
    {
        self.animation_fn = Box::new(animation_fn);
    }

    /// Set a custom error animation function for this view, indicating that the wrapped view
    /// has failed to load. See the `default_error` function reference for an example on
    /// how to create a custom error animation function.
    ///
    /// This function may be set at any time. The error animation can be changed even if
    /// the previous error animation has already started.
    pub fn set_error_fn<F>(&mut self, error_fn: F)
    where
        F: Fn(&str, usize, usize, usize, usize) -> AnimationFrame + 'static,
    {
        self.error_fn = Box::new(error_fn);
    }

    /// Make the loading animation inherit its width from the parent view. This is the default.
    pub fn inherit_width(&mut self) {
        self.width = None;
    }

    /// Make the loading animation inherit its height from the parent view. This is the default.
    pub fn inherit_height(&mut self) {
        self.height = None;
    }
}

impl<T: View> Drop for AsyncView<T> {
    fn drop(&mut self) {
        match self.error_sender.send(()) {
            Ok(_) => {}
            Err(send_err) => warn!(
                "Refreshing thread has been dropped before view has, this has no impact on your code and is a bug: {}",
                send_err
            ),
        }
    }
}

impl<T: View + Sized> View for AsyncView<T> {
    fn draw(&self, printer: &Printer) {
        match self.view {
            AsyncState::Available(ref view) => view.draw(printer),
            _ => self.loading.draw(printer),
        }
    }

    fn layout(&mut self, vec: Vec2) {
        match self.view {
            AsyncState::Available(ref mut view) => view.layout(vec),
            _ => self.loading.layout(vec),
        }
    }

    fn needs_relayout(&self) -> bool {
        match self.view {
            AsyncState::Available(ref view) => view.needs_relayout(),
            _ => true,
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        match self.rx.try_recv() {
            Ok(view) => {
                if let AsyncState::Error(_) = view {
                    self.error_idx = self.pos;
                }

                self.view = view;
            }
            Err(TryRecvError::Empty) => {
                // if empty, try next tick
            }
            Err(TryRecvError::Disconnected) => {
                // if disconnected, view is loaded or error message is displayed
            }
        }

        match self.view {
            AsyncState::Available(ref mut view) => view.required_size(constraint),
            AsyncState::Error(ref msg) => {
                let width = self.width.unwrap_or(constraint.x);
                let height = self.height.unwrap_or(constraint.y);

                let AnimationFrame {
                    content,
                    next_frame_idx,
                } = (self.error_fn)(msg, width, height, self.error_idx, self.pos);
                self.loading.set_content(content);
                self.pos = next_frame_idx;

                self.loading.required_size(constraint)
            }
            AsyncState::Pending => {
                let width = self.width.unwrap_or(constraint.x);
                let height = self.height.unwrap_or(constraint.y);

                let AnimationFrame {
                    content,
                    next_frame_idx,
                } = (self.animation_fn)(width, height, self.pos);
                self.loading.set_content(content);
                self.pos = next_frame_idx;

                self.loading.required_size(constraint)
            }
        }
    }

    fn on_event(&mut self, ev: Event) -> EventResult {
        match self.view {
            AsyncState::Available(ref mut view) => view.on_event(ev),
            _ => EventResult::Ignored,
        }
    }

    fn call_on_any<'a>(&mut self, sel: &Selector, cb: AnyCb<'a>) {
        if let AsyncState::Available(ref mut view) = self.view {
            view.call_on_any(sel, cb)
        }
    }

    fn focus_view(&mut self, sel: &Selector) -> Result<(), ()> {
        match self.view {
            AsyncState::Available(ref mut view) => view.focus_view(sel),
            _ => Err(()),
        }
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match self.view {
            AsyncState::Available(ref mut view) => view.take_focus(source),
            _ => false,
        }
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        match self.view {
            AsyncState::Available(ref view) => view.important_area(view_size),
            _ => self.loading.important_area(view_size),
        }
    }
}
