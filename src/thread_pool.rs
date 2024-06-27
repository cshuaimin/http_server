use std::{
    mem,
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::error::Result;

#[derive(Debug)]
pub struct ThreadPool<T> {
    sender: Option<mpsc::SyncSender<T>>,
    threads: Vec<thread::JoinHandle<()>>,
}

type ThreadFunc<T> = fn(&mut String, T) -> Result<()>;

impl<T> ThreadPool<T>
where
    T: Send + 'static,
{
    pub fn new(num_threads: usize, func: ThreadFunc<T>) -> Self {
        let (tx, rx) = mpsc::sync_channel(num_threads);
        let rx = Arc::new(Mutex::new(rx));

        let threads = (0..num_threads)
            .map(|_| {
                thread::spawn({
                    let rx = Arc::clone(&rx);
                    move || {
                        // Per-thread buffer.
                        let mut buf = String::new();
                        loop {
                            // Make sure to unlock the mutex before running the callback.
                            let msg = rx.lock().unwrap().recv();
                            match msg {
                                Ok(arg) => {
                                    if let Err(err) = func(&mut buf, arg) {
                                        eprintln!("Error when handling connection: {err}");
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                })
            })
            .collect();

        ThreadPool {
            sender: Some(tx),
            threads,
        }
    }

    pub fn run(&self, arg: T) {
        if let Some(tx) = &self.sender {
            tx.send(arg).unwrap();
        }
    }
}

impl<T> Drop for ThreadPool<T> {
    fn drop(&mut self) {
        self.sender.take();
        for thread in mem::take(&mut self.threads) {
            thread.join().unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::{
            atomic::{AtomicU32, Ordering},
            Arc,
        },
        thread,
        time::{Duration, Instant},
    };

    use super::ThreadPool;

    #[test]
    fn test_run_concurrently() {
        let pool = ThreadPool::new(10, |_, counter: Arc<AtomicU32>| {
            thread::sleep(Duration::from_micros(200));
            counter.fetch_add(1, Ordering::Relaxed);
            Result::Ok(())
        });

        let counter = Arc::new(AtomicU32::new(0));
        let start = Instant::now();
        for _ in 0..10 {
            pool.run(Arc::clone(&counter));
        }
        drop(pool);
        let elapsed = (Instant::now() - start).as_micros();

        assert_eq!(counter.load(Ordering::Relaxed), 10);
        assert!(
            (200..1000).contains(&elapsed),
            "should run in parallel: {elapsed}"
        );
    }
}
