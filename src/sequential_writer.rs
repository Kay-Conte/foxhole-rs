use std::{
    io::Write,
    sync::mpsc::{channel, Receiver, Sender},
};

pub enum State<W> {
    Writer(W),
    Waiting(Receiver<W>),
}

/// A synchronization type to order writes to a writer.
pub struct SequentialWriter<W>
where
    W: Write + Send,
{
    state: State<W>,
    next: Sender<W>,
}

impl<W> SequentialWriter<W>
where
    W: Write + Send + Sync,
{
    pub fn new(state: State<W>) -> (Self, Receiver<W>) {
        let (sender, receiver) = channel();

        (
            Self {
                state,
                next: sender,
            },
            receiver,
        )
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
        let mut writer = match self.state {
            State::Writer(w) => w,
            State::Waiting(r) => r.recv().expect("Failed to get writer from the receiver"),
        };

        writer.write_all(bytes)?;

        writer.flush()?;

        let _ = self.next.send(writer);

        Ok(())
    }
}
