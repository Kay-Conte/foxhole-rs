use std::sync::mpsc::{Receiver, channel, Sender};

pub enum Lazy<T> {
    Receiver(Receiver<T>),
    Body(T)
}

impl<T> Lazy<T> {
    pub fn new() -> (Self, Sender<T>) {
        let (sender, receiver) = channel();

        (Lazy::Receiver(receiver), sender)
    }

    pub fn get(&mut self) -> &T {
        use Lazy::*;

        match self {
            Receiver(r) => {
                let body = r.recv().unwrap();
                
                *self = Body(body);

                self.get()
            }

            Body(b) => b
        }
    }
}
