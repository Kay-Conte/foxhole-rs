use std::{
    io::{Write, Read},
    sync::mpsc::{channel, Receiver, Sender},
};

pub enum State<W> {
    Writer(W),
    Waiting(Receiver<W>),
}

/// A synchronization type to order writes to a writer.
pub struct SequentialStream<W>
where
    W: Read + Write + Send + Sync,
{
    state: State<W>,
    next: Sender<W>,
}

impl<S> SequentialStream<S>
where
    S: Read + Write + Send + Sync,
{
    pub fn new(state: State<S>) -> (Self, Receiver<S>) {
        let (sender, receiver) = channel();

        (
            Self {
                state,
                next: sender,
            },
            receiver,
        )
    }

    pub fn take_stream(self) -> S {
        match self.state {
            State::Writer(w) => w,
            State::Waiting(r) => r.recv().expect("Failed to get writer from the receiver"),
        }
    }


    /// # Blocks
    ///
    /// This function blocks while waiting to receive the writer handle. This has the potential to
    /// block indefinitely in the case where the `SequentialWriter` is never written to.
    ///
    /// # Panics
    ///
    /// This function should only panic if the previous `Sender` has closed without sending a
    /// writer
    pub fn send(self, bytes: &[u8]) -> std::io::Result<()> {
        let mut stream = match self.state {
            State::Writer(w) => w,
            State::Waiting(r) => r.recv().expect("Failed to get writer from the receiver"),
        };

        stream.write_all(bytes)?;

        stream.flush()?;

        let _ = self.next.send(stream);

        Ok(())
    }
}
