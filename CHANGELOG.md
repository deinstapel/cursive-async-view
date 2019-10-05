# Changelog

## v 0.2.0

### :book: Conceptual Changes

Version 0.2.0 reworks how `cursive-async-view` works at its core. Instead of spawning a separate thread to execute the creator in, it now runs a given `poll_ready` callback every event loop to let it check from different sources if the view can be created.
If this is the case the view is returned in an `AsyncState`, which itself is an enum indicating the different states the creation may have.  
Note that `poll_ready` is called until it resolves into either `AsyncState::Available` or `AsyncState::Error`.

All this is done to avoid requiring views to implement the `Send` trait. The child view is always created in the **same** thread as the cursive event loop runs.

### :pencil: API Changes

#### AsyncView

 - `AsyncView::new` has been changed to use a `poll_ready` callback to accomodate working without threads, returning an `AsyncState` instead of the view.
 - `AsyncView::new_with_bg_creator` is a convenience wrapper for `AsyncView::new` where a worker thread is spawned for a data creation function (`bg_task`). The produced data is passed to a view creation function which creates the view on the cursive thread. Further explanation is below.
 - `AsyncView::with_error_fn` & `AsyncView::set_error_fn` have been added, allowing the modification of the newly introduced error animation, in case the view creation fails.
 
#### AsyncProgressView

`AsyncProgressView` has been changed in a similar manner, again to allow creating views in the cursive thread
 - In `AsyncProgressView::new` `poll_ready` has been modified to return `AsyncProgressState` instead of the created view, and no longer receives a `Sender` as parameter. `AsyncProgressState` is quite similar to `AsyncState` as it has `Available`, `Error(String)` and `Pending(...)` variant but the pending has been extended to take a value of type `f32`, which indicates the progress that has been made.
 - The signature of the `progress_fn` has been changed to allow more complex animations!
 - Because of this the signature of of the `default_progress` has been modified to display this.
 - A new animation for progress!
 
## :package: 0.2.0 Migration

#### AsyncView

If you want to use the new version, be aware that all of the above mentioned are breaking changes, and have to be treated.

While this example was valid code for `^0.1` it has to be modified to work with the new version:

```rust
use crossbeam::Sender;
use cursive::{views::TextView, Cursive};
use cursive_async_view::AsyncView;

let mut siv = Cursive::default();
let async_view = AsyncView::new(&siv, || {
    std::thread::sleep(std::time::Duration::from_secs(5));
    TextView::new("Yay, the content has loaded!")
});

siv.add_layer(async_view);
siv.run();
```

We can do this by splitting our creation function into to two. One that creates the data (a string with the content `"Yay, the content has loaded!"`) and one that creates the view (TextView).
For situation like this where we have a creator for our data, and one for our view, we can use `new_with_bg_creator` for an easier creation.

```rust
use cursive::{views::TextView, Cursive};
use cursive_async_view::AsyncView;

let mut siv = Cursive::default();
let async_view = AsyncView::new_with_bg_creator(
    &mut siv,
    || -> Result<String, String> {
        std::thread::sleep(std::time::Duration::from_secs(5));
        Ok("Yay, the content has loaded!")
    },
    TextView::new,
);
siv.add_layer(async_view);
siv.run();
```

If you do not want to use this abstraction, you can also create an instance of async view without the background creator and check in your `poll_ready` if results are ready.
Again if you wait for some data, which would result in a blocking operation, you can spawn a thread that in which you perform the operation.
To do this you need some form of communication between the threads, for this you can use `crossbeam` and create a channel with receiver and sender and share this between the created thread and your `poll_ready`.

```rust
use crossbeam::unbounded;
use cursive::{views::TextView, Cursive};
use cursive_async_view::{AsyncState, AsyncView};

let (sender, receiver) = unbounded();
std::thread::spawn(move || {
    std::thread::sleep(std::time::Duratiom::from_secs(5));
    sender.send("Yay, the content has loaded!").unwrap();
});

let mut siv = Cursive::default();
let async_view = AsyncView::new(&mut siv, move || -> AsyncState {
    match receiver.try_recv() {
        Ok(msg) => AsyncState::Available(TextView::new(msg)),
        Err(_) => AsyncState::Pending,
    }
});

siv.add_layer(async_view);
siv.run();
```

#### AsyncProgressView

Similar to `AsyncView` `AsyncProgressView` also needs to be migrated by hand, since breaking API Changes took place.
The return type of the `poll_ready` has been changed to `AsyncProgressState`. The values are explained in `API Changes`.

```rust 
use crossbeam::Sender;
use cursive::{views::TextView, Cursive};
use cursive_async_view::AsyncProgressView;

let mut siv = Cursive::default();
let async_view = AsyncProgressView::new(&siv, |s: Sender<f32>| {
    std::thread::sleep(std::time::Duration::from_secs(1));
    s.send(0.2).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    s.send(0.4).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    s.send(0.6).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    s.send(0.8).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    s.send(1.0).unwrap();
    TextView::new("Yay, the content has loaded!")
});

siv.add_layer(async_view);
siv.run();
```

As we saw in the changes, we no longer receive a `Sender` in our `poll_ready` instead we have to tell `AsyncProgressView` via the returned state how far our creation has come.

And also in this application it is important that we split up our creation from the blocking part (in this example simply waiting) and the view creation (TextView).

```rust 
use cursive::{views::TextView, Cursive};
use cursive_async_view::{AsyncProgressView, AsyncProgressState};
use crossbeam::unbounded;

let mut siv = Cursive::default();
let (sender, receiver) = unbounded();

let wait_time = std::time::Duration::from_secs(1);

std::thread::spawn(move || {
    std::thread::sleep(wait_time);
    sender.send(0.2);
    std::thread::sleep(wait_time);
    sender.send(0.4);
    std::thread::sleep(wait_time);
    sender.send(0.6);
    std::thread::sleep(wait_time);
    sender.send(0.8);
    std::thread::sleep(wait_time);
    sender.send(1.0);
});

let async_view = AsyncProgressView::new(&mut siv, move || {
    match receiver.try_recv() {
        Ok(val) => {
            if val == 1.0 {
                AsyncProgressState::Available(TextView::new("Yay, the content has loaded"))
            } else {
                AsyncProgressState::Pending(val)
            }
        },
        Err(_) => AsyncProgressState::Error("Oh no, an error occured."),
    }
});

siv.add_layer(async_view);
siv.run();
```
