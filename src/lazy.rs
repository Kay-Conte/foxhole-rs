use std::{
    cell::OnceCell,
    sync::mpsc::{channel, Receiver, Sender},
};

pub struct Lazy<T> {
    receiver: Receiver<T>,
    value: OnceCell<T>,
}

impl<T> Lazy<T> {
    /// Constructs a new instance of `Lazy` and returns it's corresponding `Sender`
    pub fn new() -> (Self, Sender<T>) {
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
    pub fn get<'a>(&'a self) -> &T {
        self.value.get_or_init(|| {
            self.receiver.recv().unwrap()
        })
    }
}
