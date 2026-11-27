use crate::session::Ticket;
use obdium::scalar::{Scalar, Unit};
use std::io;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};

pub(crate) struct Completion {
    pub(crate) ticket: Ticket,
    pub(crate) sample: Scalar,
}

/// One worker, one queued command and one queued completion.
/// Only synthetic observations are permitted in this experiment.
pub(crate) struct DemoWorker {
    commands: SyncSender<Ticket>,
    completions: Receiver<Completion>,
}

impl DemoWorker {
    pub(crate) fn spawn() -> io::Result<(Self, JoinHandle<()>)> {
        let (commands, requests) = mpsc::sync_channel::<Ticket>(1);
        let (replies, completions) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("obdium-synthetic-demo".into())
            .spawn(move || {
                while let Ok(ticket) = requests.recv() {
                    let sample = Scalar::new(1726.0, Unit::RPM, None);
                    // Backpressure is confined to the worker. Dropping the
                    // receiver during shutdown unblocks this send.
                    if replies.send(Completion { ticket, sample }).is_err() {
                        break;
                    }
                }
            })?;
        Ok((
            Self {
                commands,
                completions,
            },
            thread,
        ))
    }

    pub(crate) fn submit(&self, ticket: Ticket) -> Result<(), TrySendError<Ticket>> {
        self.commands.try_send(ticket)
    }

    pub(crate) fn receive(&self) -> Result<Completion, TryRecvError> {
        self.completions.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::PreviewSession;
    use std::time::{Duration, Instant};

    #[test]
    fn worker_returns_correlated_synthetic_sample_and_shuts_down() {
        let (worker, thread) = DemoWorker::spawn().expect("worker");
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(Instant::now()).expect("ticket");
        worker.submit(ticket).expect("submit");
        let reply = worker
            .completions
            .recv_timeout(Duration::from_secs(10))
            .expect("reply");
        assert_eq!(reply.ticket, ticket);
        assert_eq!(reply.sample.unit, Unit::RPM);
        assert_eq!(reply.sample.value, 1726.0);
        drop(worker);
        thread.join().expect("shutdown");
    }

    #[test]
    fn full_queue_and_closed_channels_are_explicit_and_nonblocking() {
        let (commands, requests) = mpsc::sync_channel(1);
        let (replies, completions) = mpsc::sync_channel(1);
        let worker = DemoWorker {
            commands,
            completions,
        };
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(Instant::now()).expect("ticket");
        assert!(worker.submit(ticket).is_ok());
        assert!(matches!(worker.submit(ticket), Err(TrySendError::Full(_))));
        assert!(matches!(worker.receive(), Err(TryRecvError::Empty)));
        drop(requests);
        drop(replies);
        assert!(matches!(
            worker.submit(ticket),
            Err(TrySendError::Disconnected(_))
        ));
        assert!(matches!(worker.receive(), Err(TryRecvError::Disconnected)));
    }

    #[test]
    fn shutdown_releases_worker_even_with_unconsumed_replies() {
        let (worker, thread) = DemoWorker::spawn().expect("worker");
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(Instant::now()).expect("ticket");
        worker.submit(ticket).expect("first submit");
        // Wait for the worker to consume the command, without relying on a
        // sleep or scheduling assumptions. The next two replies may remain queued.
        worker
            .completions
            .recv_timeout(Duration::from_secs(10))
            .expect("first reply");
        worker.submit(ticket).expect("second submit");
        drop(worker);
        thread.join().expect("shutdown");
    }
}
