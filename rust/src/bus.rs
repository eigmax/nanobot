use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::messages::{InboundMessage, OutboundMessage};

/// Async message bus that decouples chat channels from the agent core.
///
/// Channels push messages to the inbound queue, and the agent processes
/// them and pushes responses to the outbound queue.
#[pyclass]
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<InboundMessage>>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<OutboundMessage>>>,
    running: Arc<AtomicBool>,
    inbound_count: Arc<AtomicUsize>,
    outbound_count: Arc<AtomicUsize>,
}

#[pymethods]
impl MessageBus {
    #[new]
    fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        Self {
            inbound_tx,
            inbound_rx: Arc::new(tokio::sync::Mutex::new(inbound_rx)),
            outbound_tx,
            outbound_rx: Arc::new(tokio::sync::Mutex::new(outbound_rx)),
            running: Arc::new(AtomicBool::new(false)),
            inbound_count: Arc::new(AtomicUsize::new(0)),
            outbound_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Publish a message from a channel to the agent.
    fn publish_inbound<'py>(
        &self,
        py: Python<'py>,
        msg: InboundMessage,
    ) -> PyResult<Bound<'py, PyAny>> {
        let tx = self.inbound_tx.clone();
        let count = self.inbound_count.clone();

        future_into_py(py, async move {
            tx.send(msg)
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Inbound queue closed"))?;
            count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
    }

    /// Consume the next inbound message (blocks until available).
    fn consume_inbound<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rx = self.inbound_rx.clone();
        let count = self.inbound_count.clone();

        future_into_py(py, async move {
            let mut guard = rx.lock().await;
            match guard.recv().await {
                Some(msg) => {
                    count.fetch_sub(1, Ordering::Relaxed);
                    Ok(msg)
                }
                None => Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "Inbound queue closed",
                )),
            }
        })
    }

    /// Publish a response from the agent to channels.
    fn publish_outbound<'py>(
        &self,
        py: Python<'py>,
        msg: OutboundMessage,
    ) -> PyResult<Bound<'py, PyAny>> {
        let tx = self.outbound_tx.clone();
        let count = self.outbound_count.clone();

        future_into_py(py, async move {
            tx.send(msg)
                .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Outbound queue closed"))?;
            count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
    }

    /// Consume the next outbound message (blocks until available).
    fn consume_outbound<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rx = self.outbound_rx.clone();
        let count = self.outbound_count.clone();

        future_into_py(py, async move {
            let mut guard = rx.lock().await;
            match guard.recv().await {
                Some(msg) => {
                    count.fetch_sub(1, Ordering::Relaxed);
                    Ok(msg)
                }
                None => Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "Outbound queue closed",
                )),
            }
        })
    }

    /// Stop the dispatcher loop.
    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Number of pending inbound messages.
    #[getter]
    fn inbound_size(&self) -> usize {
        self.inbound_count.load(Ordering::Relaxed)
    }

    /// Number of pending outbound messages.
    #[getter]
    fn outbound_size(&self) -> usize {
        self.outbound_count.load(Ordering::Relaxed)
    }

    fn __repr__(&self) -> String {
        format!(
            "MessageBus(inbound_size={}, outbound_size={})",
            self.inbound_size(),
            self.outbound_size()
        )
    }
}
