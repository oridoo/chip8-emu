use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct DelayTimer {
    value: u8,
    start_time: Instant,
    interval: Duration,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl DelayTimer {
    pub fn new() -> Self {
        Self {
            value: 0,
            start_time: Instant::now(),
            interval: Duration::from_millis(16), // 60 Hz
            thread_handle: None,
        }
    }

    pub fn set_value(&mut self, value: u8) {
        self.value = value;
        self.start_time = Instant::now();
    }

    pub fn get_value(&self) -> u8 {
        let elapsed = self.start_time.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        let value = if elapsed_ms > (self.value as f64 * self.interval.as_secs_f64() * 1000.0) {
            0
        } else {
            self.value - (elapsed_ms / self.interval.as_secs_f64() / 1000.0) as u8
        };
        value
    }

    pub fn start(&mut self) {
        let dt = Arc::new(Mutex::new(self.clone()));
        let handle = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(16));
                let mut dt = dt.lock().unwrap();
                if dt.value > 0 {
                    dt.value -= 1;
                }
            }
        });
        self.thread_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            handle.join().unwrap();
        }
    }
}

impl Clone for DelayTimer {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            start_time: self.start_time,
            interval: self.interval,
            thread_handle: None,
        }
    }
}

impl Drop for DelayTimer {
    fn drop(&mut self) {
        self.stop();
    }
}