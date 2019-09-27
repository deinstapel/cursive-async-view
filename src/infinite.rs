use crossbeam::channel::{self, Sender, Receiver, TryRecvError};
use cursive::direction::Direction;
use cursive::event::{AnyCb, Event, EventResult};
use cursive::theme::PaletteColor;
use cursive::utils::markup::StyledString;
use cursive::view::{Selector, View};
use cursive::views::TextView;
use cursive::{Cursive, Printer, Rect, Vec2};
use interpolation::Ease;
use num::clamp;
use failure::Fail;
use send_wrapper::SendWrapper;

use crate::utils;

/// This struct represents the content of a single loading animation frame,
/// produced by a animation function of the `AsyncView`. Read the documentation
/// of the `default_animation` to see how to implement your own animation function.
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
/// use cursive::Cursive;
/// use cursive::views::TextView;
/// use cursive::utils::markup::StyledString;
/// use cursive_async_view::{AsyncView, AnimationFrame};
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
/// let async_view = AsyncView::new(&siv, move || {
///     std::thread::sleep(std::time::Duration::from_secs(10));
///     TextView::new("Yay!\n\nThe content has loaded!")
/// })
/// .with_animation_fn(my_loading_animation);
/// ```
///
/// This animation function will first display `loading` for 1 second and then display
/// `content` for 1 second.
///
/// The `width` and `height` parameters contain the maximum size the content may have
/// (in characters). The initial `frame_idx` is 0.
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

#[derive(Debug, Fail)]
pub enum HandleError {
    #[fail(display = "The async view receiving the loaded view is not available (e.g. already dropped)")]
    ViewNotAvailable,
}

pub struct AsyncHandle<V: View> {
    // TODO: use a sender reference here to force the handle user to be on the same thread as the async-view. Also, this would eliminate the ViewNotAvailable error as the compiler could check for the lifetime.
    chan: Sender<AsyncState<V>>,
}

enum AsyncState<V: View> {
    Loaded(V),
    Error(String),
    Pending,
}

impl<V: View> AsyncHandle<V> {
    pub fn loaded(self, view: V) -> Result<(), HandleError> {
        self.chan.send(AsyncState::Loaded(view))
            .map_err(|_| HandleError::ViewNotAvailable)
    }

    pub fn error<S: Into<String>>(self, msg: S) -> Result<(), HandleError> {
        self.chan.send(AsyncState::Error(msg.into()))
            .map_err(|_| HandleError::ViewNotAvailable)
    }
}

/// An `AsyncView` is a wrapper view that displays a loading screen, until the child
/// view is successfully created. The creation of the inner view is done on a
/// dedicated thread. Therefore, it is necessary for the creation function to
/// always return, otherwise the thread will get stuck.
///
/// # Example usage
///
/// ```
/// use cursive::{views::TextView, Cursive};
/// use cursive_async_view::AsyncView;
///
/// let mut siv = Cursive::default();
/// let async_view = AsyncView::new(&siv, move || {
///     std::thread::sleep(std::time::Duration::from_secs(10));
///     TextView::new("Yay!\n\nThe content has loaded!")
/// });
///
/// siv.add_layer(async_view);
/// // siv.run();
/// ```
///
/// The content will be displayed after 10 seconds.
///
/// # Threads
///
/// The `new(siv, creator)` method will spawn 2 threads:
///
/// 1. `cursive-async-view::creator` The creation thread for the wrapped view.
///    This thread will stop running as soon as the creation function returned.
/// 2. `cursive-async-view::updater` The update thread for ensuring 30fps during
///    the loading animation. This thread will be stopped by `AsyncView` when the
///    creation function returned and the new view is available for layouting.
///
/// The threads are labeled as indicated above.
///
/// # TODO
///
/// * make creation function return a result to mark an unsuccessful creation
///
pub struct AsyncView<T: View> {
    view: AsyncState<T>,
    loading: TextView,
    animation_fn: Box<dyn Fn(usize, usize, usize) -> AnimationFrame + 'static>,
    width: Option<usize>,
    height: Option<usize>,
    pos: usize,
    tx: Sender<AsyncState<T>>,
    rx: Receiver<AsyncState<T>>,
}

impl<T: View> AsyncView<T> {
    /// Create a new `AsyncView` instance. The cursive reference is only used
    /// to control the refresh rate of the terminal when the loading animation
    /// is running. In order to show the view, it has to be directly or indirectly
    /// added to a cursive layer like any other view.
    ///
    /// The creator function will be executed on a dedicated thread in the
    /// background. Make sure that this function will never block indefinitely.
    /// Otherwise, the creation thread will get stuck.
    pub fn new(siv: &Cursive) -> Self
    {
        // trust me, I'm an engineer
        let (tx, rx) = channel::unbounded();

        //Self::waiting_cb(siv);

        Self {
            view: AsyncState::Pending,
            loading: TextView::new(""),
            animation_fn: Box::new(default_animation),
            width: None,
            height: None,
            pos: 0,
            rx,
            tx,
        }
    }

    pub fn handle(&self) -> SendWrapper<AsyncHandle<T>> {
        SendWrapper::new(AsyncHandle {
            chan: self.tx.clone(),
        })
    }

    fn waiting_cb(siv: &Cursive) {
        siv.cb_sink().send(Box::new(|siv| Self::waiting_cb(siv))).unwrap();
    }

    /// Mark the maximum allowed width in characters, the loading animation may consume.
    /// By default, the width will be inherited by the parent view.
    pub fn with_width(self, width: usize) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    /// Mark the maximum allowed height in characters, the loading animation may consume.
    /// By default, the height will be inherited by the parent view.
    pub fn with_height(self, height: usize) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    /// Set a custom animation function for this view, indicating that the wrapped view is
    /// not available yet. See the `default_animation` function reference for an example on
    /// how to create a custom animation function.
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
            pos: 0,
            animation_fn: Box::new(animation_fn),
            ..self
        }
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
    ///
    /// > The `frame_idx` of the loading animation is reset to 0 when setting a new animation function
    pub fn set_animation_fn<F>(&mut self, animation_fn: F)
    where
        F: Fn(usize, usize, usize) -> AnimationFrame + 'static,
    {
        self.pos = 0;
        self.animation_fn = Box::new(animation_fn);
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

impl<T: View + Sized> View for AsyncView<T> {
    fn draw(&self, printer: &Printer) {
        match self.view {
            AsyncState::Loaded(ref view) => view.draw(printer),
            AsyncState::Error(ref msg) => TextView::new(msg).draw(printer),
            AsyncState::Pending => self.loading.draw(printer),
        }
    }

    fn layout(&mut self, vec: Vec2) {
        match self.view {
            AsyncState::Loaded(ref mut view) => view.layout(vec),
            AsyncState::Error(_) => {},
            AsyncState::Pending => self.loading.layout(vec),
        }
    }

    fn needs_relayout(&self) -> bool {
        match self.view {
            AsyncState::Loaded(ref view) => view.needs_relayout(),
            _ => true,
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        match self.rx.try_recv() {
            Ok(view) => {
                self.view = view;
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => unreachable!(),
        }


        match self.view {
            AsyncState::Loaded(ref mut view) => view.required_size(constraint),
            AsyncState::Error(ref msg) => TextView::new(msg).required_size(constraint),
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
            AsyncState::Loaded(ref mut view) => view.on_event(ev),
            _ => EventResult::Ignored,
        }
    }

    fn call_on_any<'a>(&mut self, sel: &Selector, cb: AnyCb<'a>) {
        match self.view {
            AsyncState::Loaded(ref mut view) => view.call_on_any(sel, cb),
            _ => {},
        }
    }

    fn focus_view(&mut self, sel: &Selector) -> Result<(), ()> {
        match self.view {
            AsyncState::Loaded(ref mut view) => view.focus_view(sel),
            _ => Err(()),
        }
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        match self.view {
            AsyncState::Loaded(ref mut view) => view.take_focus(source),
            _ => false,
        }
    }

    fn important_area(&self, view_size: Vec2) -> Rect {
        match self.view {
            AsyncState::Loaded(ref view) => view.important_area(view_size),
            AsyncState::Error(ref msg) => TextView::new(msg).important_area(view_size),
            AsyncState::Pending => self.loading.important_area(view_size),
        }
    }
}
