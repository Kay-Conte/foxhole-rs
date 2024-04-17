use std::sync::{
    mpsc::{self, Receiver, SendError},
    Arc,
};

use mio::Waker;

pub(crate) struct SyncSender<T> {
    waker: Arc<Waker>,
    sender: std::sync::mpsc::SyncSender<T>,
}

impl<T> Clone for SyncSender<T> {
    fn clone(&self) -> Self {
        Self::new(self.waker.clone(), self.sender.clone())
    }
}

impl<T> SyncSender<T> {
    fn new(waker: Arc<Waker>, sender: std::sync::mpsc::SyncSender<T>) -> Self {
        Self { waker, sender }
    }

    pub(crate) fn send(&mut self, value: T) -> Result<(), SendError<T>> {
        let res = self.sender.send(value);

        self.waker.wake().unwrap();

        res
    }
}

pub(crate) fn sync_channel<T>(waker: Arc<Waker>, bound: usize) -> (SyncSender<T>, Receiver<T>) {
    let (sender, receiver) = mpsc::sync_channel(bound);

    (SyncSender::new(waker, sender), receiver)
}
