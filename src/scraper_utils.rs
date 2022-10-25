use std::sync::Mutex;
use std::time::Instant;

use lazy_static::lazy_static;

lazy_static! {
    static ref LAST_REQUEST_MUTEX: Mutex<Option<Instant>> = Mutex::new(None);
    static ref REQUEST_DELAY: std::time::Duration = std::time::Duration::from_millis(500);
}

// Add a minimum time delay so as not to overload the server
pub fn throttle() {
    let mut last_request_mutex = LAST_REQUEST_MUTEX.lock().unwrap();

    let last_request = last_request_mutex.take();
    let now = Instant::now();
    if let Some(last_request) = last_request {
        let duration = now.duration_since(last_request);
        if duration < *REQUEST_DELAY {
            std::thread::sleep(*REQUEST_DELAY - duration);
        }
    }
    last_request_mutex.replace(now);
}
