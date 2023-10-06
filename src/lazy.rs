use std::sync::mpsc::{Receiver, channel, Sender};

pub enum Lazy<T> {
    Receiver(Receiver<T>),
    Value(T)
}

impl<T> Lazy<T> {
    /// Constructs a new instance of `Lazy` and returns it's corresponding `Sender`
    pub fn new() -> (Self, Sender<T>) {
        let (sender, receiver) = channel();

        (Lazy::Receiver(receiver), sender)
    }

    /// This call blocks until the body has been read from the `TcpStream`
    ///
    /// # Panics
    ///
    /// This call will panic if its corresponding `Sender` hangs up before sending a value
    pub fn get(&mut self) -> &T {
        use Lazy::*;

        match self {
            Receiver(r) => {
                let body = r.recv().unwrap();
                
                *self = Value(body);

                self.get()
            }

            Value(b) => b
        }
    }
}
