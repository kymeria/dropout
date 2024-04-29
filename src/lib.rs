//! Dropout allow to send an object in a background thread to be dropped there.
//!
//! Dropout is inspired by [defer-drop](https://docs.rs/defer-drop) and (as defer-drop itself) by [https://abramov.io/rust-dropping-things-in-another-thread](https://abramov.io/rust-dropping-things-in-another-thread)
//!
//! See [`Dropper`] for details.

use std::sync::Arc;

/// Dropper can send object to a background thread to be dropped there.
/// Useful when the object takes a long time to drop and you don't want your (main) thread
/// to be blocked while you drop it.
///
/// # Notes:
///
/// There is one dropper thread per `Dropper`. Dropped values are enqueued in an
/// unbounded channel to be consumed by this thread; if you send more
/// value than the thread can handle, this will cause unbounded memory
/// consumption. There is currently no way for the thread to signal or block
/// if it is overwhelmed.
///
/// The objects are guaranteed to be destructed in the order received through a
/// channel, which means that objects sent from a single thread will be
/// destructed in order. However, there is no guarantee about the ordering of
/// interleaved values from different threads.
/// Value send to be dropped are guaranted to be dropped at a moment as `Dropper` itself
/// wait for all values to be dropped when it is been dropped.
///
/// # Example
///
/// ```
/// # use std::time::{Instant, Duration};
/// # use dropout::Dropper;
/// # use std::collections::HashMap;
///
///
/// # type HeavyObject = HashMap<usize, Vec<usize>>;
///
/// # const NUM_ELEMENTS: usize = 1000000;
/// # fn make_heavy_object() -> HeavyObject {
/// #    (1..=NUM_ELEMENTS).map(|v| (v, vec![v])).collect()
/// # }
///
/// let first_heavy_object = make_heavy_object();
/// let second_heavy_object = first_heavy_object.clone();
///
/// fn timer(f: impl FnOnce()) -> Duration {
///     let start = Instant::now();
///     f();
///     Instant::now() - start
/// }
///
/// let dropper = Dropper::new();
///
/// // This is a special case for this small test.
/// // The closure will explictly drop `first_heavy_object` but also implicitly drop the
/// // `dropper` and it will wait for background thread to finish.
/// // We don't want to mesure the background drop as it is what we want to avoid.
/// // By cloning the dropper, background thread is waited at end of main, not at end of closure.
/// let dropper_clone = dropper.clone();
/// let dropout_time = timer(move || dropper_clone.dropout(first_heavy_object));
/// let std_time = timer(move || drop(second_heavy_object));
///
/// assert!(dropout_time < std_time);
/// ```
pub struct Dropper<T: Send>(Arc<inner::Dropper<T>>);

impl<T: Send + 'static> Dropper<T> {
    /// Create a new Dropper.
    #[inline]
    pub fn new() -> Self {
        Self(Arc::new(inner::Dropper::new()))
    }

    /// Send a value to be dropped in another thread.
    ///
    /// If somehow the receiving part is closed (probably because of a panic in a previous object drop),
    /// `to_drop` value will be drop in the current thread.
    #[inline]
    pub fn dropout(&self, to_drop: T) {
        self.0.dropout(to_drop)
    }
}

impl<T: Send + 'static> Default for Dropper<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + 'static> Clone for Dropper<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

mod inner {
    use crossbeam_channel::{unbounded, Sender};
    use std::thread;

    pub struct Dropper<T: Send> {
        drop_sender: Option<Sender<T>>,
        thread_handle: Option<thread::JoinHandle<()>>,
    }

    impl<T: Send + 'static> Dropper<T> {
        pub fn new() -> Self {
            let (drop_sender, drop_receiver) = unbounded();
            let thread_handle = thread::Builder::new()
                .name("Dropout".into())
                .spawn(move || while let Ok(_) = drop_receiver.recv() {})
                .expect("Should succeed to create thread");
            Self {
                drop_sender: Some(drop_sender),
                thread_handle: Some(thread_handle),
            }
        }

        /// Send the object to be drop.
        ///
        /// If somehow the receiving part is closed (probably because of a panic in a previous object drop),
        /// `to_drop` will be drop in the current thread.
        #[inline]
        pub fn dropout(&self, to_drop: T) {
            let _ = self.drop_sender.as_ref().unwrap().send(to_drop);
        }
    }

    impl<T: Send> Drop for Dropper<T> {
        fn drop(&mut self) {
            drop(self.drop_sender.take());
            self.thread_handle.take().map(|h| h.join());
        }
    }
}
