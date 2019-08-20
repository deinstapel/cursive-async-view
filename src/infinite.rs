use std::thread;
use std::time::Duration;

use crossbeam::channel::{self, Receiver};
use cursive::direction::Direction;
use cursive::event::{AnyCb, Event, EventResult};
use cursive::theme::PaletteColor;
use cursive::utils::markup::StyledString;
use cursive::view::{Selector, View};
use cursive::views::TextView;
use cursive::{Cursive, Printer, Rect, Vec2};
use interpolation::Ease;
use num::clamp;

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
    /// Create a new `AsyncView` instance. The cursive reference is only used
    /// to control the refresh rate of the terminal when the loading animation
    /// is running. In order to show the view, it has to be directly or indirectly
    /// added to a cursive layer like any other view.
    ///
    /// The creator function will be executed on a dedicated thread in the
    /// background. Make sure that this function will never block indefinitely.
    /// Otherwise, the creation thread will get stuck.
    pub fn new<F>(siv: &Cursive, creator: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
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
                Err(_) => {}
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
