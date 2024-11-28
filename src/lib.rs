use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: mpsc::Sender<Message>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message { // To signal a job must work or must shutdown
  NewJob(Job),
  Terminate,
}

impl ThreadPool {
  // Create a new ThreadPool
  // size is the number of threads in the pool
  // # Panics
  // The new function will panic if the size is not greater than 0

  pub fn new(size: usize) -> ThreadPool {
    assert!(size>0); // Zero threads doesn't make sense

    let (sender, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));
    let mut workers = Vec::with_capacity(size);
    for id in 0..size {
      workers.push(Worker::new(id, Arc::clone(&receiver)));
    }
    ThreadPool { workers, sender }
  }

  pub fn execute<F>(&self, f: F)
  where
    F: FnOnce() + Send + 'static
  {
    let job = Box::new(f);
    self.sender.send(Message::NewJob(job)).unwrap();    
  }
}
impl Drop for ThreadPool {
  fn drop(&mut self) {
    println!("Sending shutdown message to all workers...");
    for _ in &self.workers {
      self.sender.send(Message::Terminate).unwrap(); // workers pick message at discretion
    }
    for worker in &mut self.workers {
      #[cfg(feature = "debug")]
      println!("Shutting down worker {}", worker.id);
      if let Some(thread) = worker.thread.take() {
        thread.join().unwrap(); // completes and end
      }
    }
  }
}

struct Worker {
  id: usize,
  thread: Option<thread::JoinHandle<()>>
}

impl Worker {
  fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
    let thread = thread::spawn( move || loop { // will always be waiting for jobs to do
      let message = receiver
        .lock()
        .unwrap()
        .recv()
        .unwrap();
      match message {
        Message::NewJob(job) => {
          #[cfg(feature = "debug")]
          println!("Worker {} got a job; executing.", id);
          job();
        }
        Message::Terminate => {
          #[cfg(feature = "debug")]
          println!("Worker {} received message to terminate.", id);
          break; // of infinite loop on thread::spawn()
        }
      }
    } );
    Worker {id, thread: Some(thread)}
  }
}

