use std::time::SystemTime;

#[cfg(not(feature = "mock-time"))]
pub fn now() -> SystemTime {
    SystemTime::now()
}

#[cfg(feature = "mock-time")]
pub mod mock_time {
    use std::cell::RefCell;

    use super::*;

    thread_local! {
        static MOCK_TIME: RefCell<Option<SystemTime>> = RefCell::new(None);
    }

    pub fn now() -> SystemTime {
        MOCK_TIME.with(|cell| {
            cell.borrow()
                .as_ref()
                .cloned()
                .unwrap_or_else(SystemTime::now)
        })
    }

    pub fn set_mock_time(time: SystemTime) {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = Some(time));
    }

    pub fn clear_mock_time() {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = None);
    }
}

#[cfg(feature = "mock-time")]
pub use mock_time::now;
