use std::{sync::{mpsc::{Sender, channel, Receiver, TryRecvError}, Arc, Mutex}, net::TcpStream, str::Split, io::{Read, Write}, collections::{VecDeque, vec_deque}};

use http::Request;

use crate::{routing::Route, http_utils::{RequestFromBytes, ResponseToBytes}, systems::RawResponse};

pub struct Task {
    pub stream: TcpStream,
    pub router: Arc<Route>,
}

pub struct Context<'b> {
    pub request: Request<Vec<u8>>,
    pub path_iter: Split<'b, &'static str>,
}

#[derive(Clone)]
struct ThreadWaker {
    channel: Sender<usize>,
}

impl ThreadWaker {
    fn new(sender: Sender<usize>) -> Self {
        Self {
            channel: sender,
        }
    }

    fn wake(&mut self, id: usize) {
        self.channel.send(id).expect("Channel hung up unexpectedly");
    }
}

pub struct ThreadPool {
    thread_channels: Vec<Sender<Task>>,
    waker: ThreadWaker,
    receiver: Receiver<usize>,
}

impl ThreadPool {
    pub fn new() -> Self {
        let cpus = num_cpus::get();

        let (sender, receiver) = channel();

        let mut pool = ThreadPool {
            thread_channels: Vec::new(),
            waker: ThreadWaker::new(sender),
            receiver,
        };

        for _ in 0..cpus {
            let id = pool.spawn_thread();

            pool.waker.wake(id);
        }

        pool
    }

    pub fn spawn_thread(&mut self) -> usize {
        let (sender, receiver): (Sender<Task>, Receiver<Task>) = channel();

        self.thread_channels.push(sender);

        let mut waker = self.waker.clone();

        let id = self.thread_channels.len() - 1;

        std::thread::spawn(move || {
            while let Ok(task) = receiver.recv() {
                handle_connection(task);

                waker.wake(id)
            }

            println!("Closing thread");
        });


        id
    }

    pub fn send_task(&mut self, task: Task) {
        let next = match self.receiver.try_recv() {
            Ok(v) => v,
            Err(TryRecvError::Empty) => self.spawn_thread(),
            Err(_) => panic!("Channel hung up unexpectedly")
        };

        self.thread_channels[next].send(task).expect("Thread hung up unexpectedly");
    } 
}

fn handle_connection(mut task: Task) {
    let mut buf = vec![0; 1024];
    let mut bytes_read: usize = 0;

    // Some left behind fragments of content-length aware reading.
    let res = task.stream.read(&mut buf);

    match res {
        Ok(n) => {
            if n == 0 {
                return;
            }

            bytes_read += n;

            let request = Request::try_from_bytes(&buf[..bytes_read]).expect("Failed to parse request");

            handle_request(task, request.map(|f| f.into_bytes()));
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return,
        Err(_) => return,
    }
}

fn handle_request(task: Task, request: Request<Vec<u8>>) {
    let path = request.uri().path().to_string();

    let mut path_iter = path.split("/");

    // Discard first in iter as it will always be an empty string
    path_iter.next();

    let mut ctx = Context {
        request,
        path_iter,
    };

    let mut cursor = task.router.as_ref();

    loop {
        for system in cursor.systems() {
            if let Some(r) = system.call(&mut ctx) {
                respond(task.stream, r);
                return;
            }
        }

        let Some(next) = ctx.path_iter.next() else {
            break;
        };

        if let Some(child) = cursor.get_child(next) {
            cursor = child;
        } else {
            break;
        }
    }
}

fn respond(mut stream: TcpStream, response: RawResponse) {
    let _ = stream.write_all(&response.into_bytes());

    let _ = stream.flush();
}
