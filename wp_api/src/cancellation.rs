use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, PoisonError},
};

use futures::StreamExt;
use futures::channel::mpsc::{TryRecvError, UnboundedSender, unbounded};

use crate::uuid::WpUuid;

#[uniffi::export(with_foreign)]
pub trait CancellationHandler: Send + Sync + std::fmt::Debug {
    fn cancelled(&self);
}

#[derive(Debug, Default, uniffi::Object)]
pub struct CancellationToken {
    uuid: WpUuid,
    cancelled: Mutex<bool>,
    waiters: Mutex<VecDeque<UnboundedSender<()>>>,
    handler: Mutex<VecDeque<Arc<dyn CancellationHandler>>>,
}

#[uniffi::export]
impl CancellationToken {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            uuid: WpUuid::default(),
            cancelled: Mutex::new(false),
            waiters: Mutex::new(VecDeque::new()),
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

    pub async fn wait_for_cancellation(&self) -> Result<(), CancellationTokenError> {
        if *self.cancelled.lock()? {
            return Ok(());
        }

        let (sender, mut receiver) = unbounded();

        {
            let mut waiters = self.waiters.lock()?;
            waiters.push_back(sender);
        }

        let _ = receiver.next().await;
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

        let mut waiters = self.waiters.lock()?;
        waiters.retain_mut(|sender| sender.unbounded_send(()).is_ok());

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

impl From<TryRecvError> for CancellationTokenError {
    fn from(_: TryRecvError) -> Self {
        CancellationTokenError::Locking
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::{sleep, timeout};

    #[tokio::test]
    async fn test_wait_for_cancellation_returns_immediately_when_already_cancelled() {
        let token = CancellationToken::new();
        token.cancel().unwrap();

        let result = token.wait_for_cancellation().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_cancellation_waits_until_cancelled() {
        let token = CancellationToken::new();
        let token_clone = std::sync::Arc::new(token);
        let token_for_task = token_clone.clone();

        let wait_task = tokio::spawn(async move { token_for_task.wait_for_cancellation().await });

        sleep(Duration::from_millis(10)).await;
        assert!(!wait_task.is_finished());

        token_clone.cancel().unwrap();

        let result = wait_task.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_cancellation_multiple_waiters() {
        let token = std::sync::Arc::new(CancellationToken::new());

        let mut tasks = Vec::new();
        for _ in 0..5 {
            let token_clone = token.clone();
            let task = tokio::spawn(async move { token_clone.wait_for_cancellation().await });
            tasks.push(task);
        }

        sleep(Duration::from_millis(10)).await;
        for task in &tasks {
            assert!(!task.is_finished());
        }

        token.cancel().unwrap();

        for task in tasks {
            let result = task.await.unwrap();
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_wait_for_cancellation_timeout_when_not_cancelled() {
        let token = CancellationToken::new();

        let result = timeout(Duration::from_millis(50), token.wait_for_cancellation()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_for_cancellation_with_concurrent_cancel() {
        let token = std::sync::Arc::new(CancellationToken::new());
        let token_for_wait = token.clone();
        let token_for_cancel = token.clone();

        let wait_task = tokio::spawn(async move { token_for_wait.wait_for_cancellation().await });

        let cancel_task = tokio::spawn(async move {
            sleep(Duration::from_millis(20)).await;
            token_for_cancel.cancel()
        });

        let wait_result = wait_task.await.unwrap();
        let cancel_result = cancel_task.await.unwrap();

        assert!(wait_result.is_ok());
        assert!(cancel_result.is_ok());
    }

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

    #[tokio::test]
    async fn test_register_handler_with_wait_for_cancellation() {
        let token = std::sync::Arc::new(CancellationToken::new());
        let (handler, called_flag) = TestHandler::new();

        token.register_handler(Arc::new(handler)).unwrap();

        let token_for_wait = token.clone();
        let wait_task = tokio::spawn(async move { token_for_wait.wait_for_cancellation().await });

        sleep(Duration::from_millis(10)).await;
        assert!(!wait_task.is_finished());
        assert!(!called_flag.load(std::sync::atomic::Ordering::SeqCst));

        token.cancel().unwrap();

        let result = wait_task.await.unwrap();
        assert!(result.is_ok());
        assert!(called_flag.load(std::sync::atomic::Ordering::SeqCst));
    }
}
