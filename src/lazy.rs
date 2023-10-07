use std::{sync::mpsc::{Receiver, channel, Sender}, cell::RefCell};

pub enum State<T> {
    Receiver(Receiver<T>),
    Value(T)
}

pub struct Lazy<T>(RefCell<State<T>>);

impl<T> Lazy<T> {
    /// Constructs a new instance of `Lazy` and returns it's corresponding `Sender`
    pub fn new() -> (Self, Sender<T>) {
        let (sender, receiver) = channel();

        (Lazy(RefCell::new(State::Receiver(receiver))), sender)
    }

    /// This call blocks until the body has been read from the `TcpStream`
    ///
    /// # Panics
    ///
    /// This call will panic if its corresponding `Sender` hangs up before sending a value
    pub fn get(&self) -> &T {
        use State::*;

        todo!();

        match *self.0.borrow() {
            Receiver(r) => {
                let body = r.recv().unwrap();
                
                self.0.replace(Value(body));

                self.get()
            }

            Value(ref b) => b
        }
    }
}
