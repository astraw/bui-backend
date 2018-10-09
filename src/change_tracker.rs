//! Tracks changes to data and notifies listeners.
use std::mem::ManuallyDrop;
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
    tx_map: Arc<Mutex<Vec<ManuallyDrop<mpsc::Sender<(T, T)>>>>>,
}

impl<T> ChangeTracker<T>
    where T: Clone + PartialEq
{
    /// Create a new `ChangeTracker` which takes ownership
    /// of the data of type `T`.
    pub fn new(value: T) -> Self {
        Self {
            value,
            tx_map: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a Receiver which will receive messages whenever a change occurs.
    ///
    /// To remove a listener, drop the Receiver.
    pub fn get_changes(&mut self) -> mpsc::Receiver<(T, T)> {
        let (tx, rx) = mpsc::channel(1);
        let tx = ManuallyDrop::new(tx);
        let mut tx_map = self.tx_map.lock();
        tx_map.push(tx);
        rx
    }

    /// Modify the value of type `T`, notifying listeners upon change.
    ///
    /// To remove a listener, drop the Receiver.
    pub fn modify<F>(&mut self, f: F)
        where F: FnOnce(&mut T)
    {
        let orig_value = self.value.clone();
        f(&mut self.value);
        let new_value = self.value.clone();
        if orig_value != new_value {
            let mut tx_map2 = self.tx_map.lock().clone();
            for on_changed_tx in tx_map2.drain(0..) {
                let mut on_changed_tx_i = ManuallyDrop::into_inner(on_changed_tx);
                on_changed_tx_i
                    .start_send((orig_value.clone(), new_value.clone())).expect("start send"); // TODO FIXME use .send() here
            }
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
