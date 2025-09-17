use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, PoisonError},
};

use crate::uuid::WpUuid;

#[uniffi::export(with_foreign)]
pub trait CancellationHandler: Send + Sync + std::fmt::Debug {
    fn cancelled(&self);
}

#[derive(Debug, Default, uniffi::Object)]
pub struct CancellationToken {
    uuid: WpUuid,
    cancelled: Mutex<bool>,
    handler: Mutex<VecDeque<Arc<dyn CancellationHandler>>>,
}

#[uniffi::export]
impl CancellationToken {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            uuid: WpUuid::default(),
            cancelled: Mutex::new(false),
            handler: Mutex::new(VecDeque::new()),
        }
    }

    pub fn uuid(&self) -> String {
        self.uuid.uuid_string()
    }

    pub fn register_handler(
        &self,
        handler: Arc<dyn CancellationHandler>,
    ) -> Result<(), CancellationTokenError> {
        let mut handlers = self.handler.lock()?;
        handlers.push_back(handler);
        Ok(())
    }

    pub fn cancel(&self) -> Result<(), CancellationTokenError> {
        let mut cancelled = self.cancelled.lock()?;
        if *cancelled {
            return Ok(());
        }

        *cancelled = true;

        let mut handlers = self.handler.lock()?;
        while let Some(handler) = handlers.pop_front() {
            handler.cancelled();
        }

        Ok(())
    }
}

#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum CancellationTokenError {
    #[error("Error acquiring lock")]
    Locking,
}

impl<T> From<PoisonError<T>> for CancellationTokenError {
    fn from(_: PoisonError<T>) -> Self {
        CancellationTokenError::Locking
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestHandler {
        called: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl TestHandler {
        fn new() -> (Self, std::sync::Arc<std::sync::atomic::AtomicBool>) {
            let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            (
                Self {
                    called: called.clone(),
                },
                called,
            )
        }
    }

    impl CancellationHandler for TestHandler {
        fn cancelled(&self) {
            self.called.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_register_handler_single_handler() {
        let token = CancellationToken::new();
        let (handler, called_flag) = TestHandler::new();

        token.register_handler(Arc::new(handler)).unwrap();

        assert!(!called_flag.load(std::sync::atomic::Ordering::SeqCst));

        token.cancel().unwrap();

        assert!(called_flag.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_register_handler_multiple_handlers() {
        let token = CancellationToken::new();
        let (handler1, called_flag1) = TestHandler::new();
        let (handler2, called_flag2) = TestHandler::new();
        let (handler3, called_flag3) = TestHandler::new();

        token.register_handler(Arc::new(handler1)).unwrap();
        token.register_handler(Arc::new(handler2)).unwrap();
        token.register_handler(Arc::new(handler3)).unwrap();

        assert!(!called_flag1.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!called_flag2.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!called_flag3.load(std::sync::atomic::Ordering::SeqCst));

        token.cancel().unwrap();

        assert!(called_flag1.load(std::sync::atomic::Ordering::SeqCst));
        assert!(called_flag2.load(std::sync::atomic::Ordering::SeqCst));
        assert!(called_flag3.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_register_handler_after_cancellation() {
        let token = CancellationToken::new();
        let (handler, called_flag) = TestHandler::new();

        token.cancel().unwrap();

        token.register_handler(Arc::new(handler)).unwrap();

        assert!(!called_flag.load(std::sync::atomic::Ordering::SeqCst));
    }
}
