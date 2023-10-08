use std::{
    cell::{Ref, RefCell},
    sync::mpsc::{channel, Receiver, Sender},
};

pub enum State<T> {
    Receiver(Receiver<T>),
    Value(T),
}

pub struct Lazy<T>(RefCell<State<T>>);

impl<T> Lazy<T> {
    /// Constructs a new instance of `Lazy` and returns it's corresponding `Sender`
    pub fn new() -> (Self, Sender<T>) {
        let (sender, receiver) = channel();

        (Lazy(RefCell::new(State::Receiver(receiver))), sender)
    }

    /// # Guarantees
    /// This function guarantees that the `State` of this `Lazy` will be of variant `Value` after
    /// this call.
    pub fn sync(&self) {
        let borrow = self.0.borrow();

        match *borrow {
            State::Receiver(ref r) => {
                let rec = r.recv().unwrap();

                drop(borrow);

                self.0.replace(State::Value(rec));
            }
            _ => {}
        }
    }

    /// This call blocks until the body has been read from the `TcpStream`
    ///
    /// # Panics
    ///
    /// This call will panic if its corresponding `Sender` hangs up before sending a value
    pub fn get<'a>(&'a self) -> Ref<'a, T> {
        self.sync();

        Ref::map(self.0.borrow(), |r| match r {
            State::Value(v) => v,
            _ => unreachable!(),
        })
    }
}
