use std::{
    cell::OnceCell,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::get_as_slice::GetAsSlice;

pub struct Lazy<T> {
    receiver: Receiver<T>,
    value: OnceCell<T>,
}

impl Lazy<Vec<u8>> {
    /// Constructs a new instance of `Lazy` and returns it's corresponding `Sender`
    pub fn new() -> (Self, Sender<Vec<u8>>) {
        let (sender, receiver) = channel();

        (
            Lazy {
                receiver,
                value: OnceCell::new(),
            },
            sender,
        )
    }

    /// This call blocks until the body has been read from the `TcpStream`
    pub fn get<'a>(&'a self) -> &[u8] {
        self.value.get_or_init(|| self.receiver.recv().unwrap())
    }
}

impl GetAsSlice for Lazy<Vec<u8>> {
    fn get_as_slice(&self) -> &[u8] {
        self.get()
    }
}
