//! Tracks changes to data and notifies listeners.
use std::sync::Arc;
use parking_lot::Mutex;
use futures::sync::mpsc;
use futures::Sink;

/// Tracks changes to data. Notifies listeners via a `futures::Stream`.
///
/// The data to be tracked is type `T`. The value of type `T` is wrapped in a
/// private field. The `AsRef` trait is implemented so `&T` can be obtained by
/// calling `as_ref()`. Read and write access can be gained by calling the
/// `modify` method.
///
/// Subsribe to changes by calling `get_changes`.
pub struct ChangeTracker<T>
    where T: Clone + PartialEq
{
    value: T,
    senders: Arc<Mutex<Vec<mpsc::Sender<(T, T)>>>>,
}

impl<T> ChangeTracker<T>
    where T: Clone + PartialEq
{
    /// Create a new `ChangeTracker` which takes ownership
    /// of the data of type `T`.
    pub fn new(value: T) -> Self {
        Self {
            value,
            senders: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a Receiver which will receive messages whenever a change occurs.
    ///
    /// To remove a listener, drop the Receiver.
    pub fn get_changes(&self) -> mpsc::Receiver<(T, T)> {
        let (tx, rx) = mpsc::channel(1);
        let mut senders = self.senders.lock();
        senders.push(tx);
        rx
    }

    /// Modify the value of type `T`, notifying listeners upon change.
    pub fn modify<F>(&mut self, f: F)
        where F: FnOnce(&mut T)
    {
        let orig = self.value.clone();
        f(&mut self.value);
        let newval = self.value.clone();
        if orig != newval {
            let mut senders = self.senders.lock();
            let mut keep = vec![];
            for mut on_changed_tx in senders.drain(0..) {
                // TODO use .send() here?
                match on_changed_tx.start_send((orig.clone(), newval.clone())) {
                    Ok(_) => keep.push(on_changed_tx),
                    Err(_) => trace!("receiver dropped"),
                }
            }
            senders.extend(keep);
        }
    }
}

impl<T> AsRef<T> for ChangeTracker<T>
    where T: Clone + PartialEq
{
    fn as_ref(&self) -> &T {
        &self.value
    }
}
