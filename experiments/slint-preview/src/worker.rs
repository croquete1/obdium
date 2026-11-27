use crate::session::{Permit, Ticket};
use obdium::scalar::{Scalar, Unit};
use std::io;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::Instant;

pub(crate) struct Completion {
    pub(crate) ticket: Ticket,
    pub(crate) sample: Scalar,
}

/// One worker, one queued command and one queued completion.
/// Only synthetic observations are permitted in this experiment.
pub(crate) struct DemoWorker {
    commands: SyncSender<Permit>,
    completions: Receiver<Completion>,
}

impl DemoWorker {
    pub(crate) fn spawn() -> io::Result<(Self, JoinHandle<()>)> {
        let (commands, requests) = mpsc::sync_channel::<Permit>(1);
        let (replies, completions) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("obdium-synthetic-demo".into())
            .spawn(move || {
                let mut source = SyntheticSource;
                while let Ok(permit) = requests.recv() {
                    let Some(sample) = observe(&mut source, &permit, Instant::now()) else {
                        continue;
                    };
                    if !permit.is_live(Instant::now()) {
                        continue;
                    }
                    // A revocation racing with send is still rejected by
                    // the receiving session. Closing the receiver unblocks send.
                    let reply = Completion {
                        ticket: permit.ticket(),
                        sample,
                    };
                    if replies.send(reply).is_err() {
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

    pub(crate) fn submit(&self, permit: Permit) -> Result<(), TrySendError<Permit>> {
        self.commands.try_send(permit)
    }

    pub(crate) fn receive(&self) -> Result<Completion, TryRecvError> {
        self.completions.try_recv()
    }
}

/// Read-only observation boundary, owned by the worker thread.
/// Production transport is deliberately not implemented here.
trait ObservationSource {
    fn read_engine_speed(&mut self) -> Scalar;
}

struct SyntheticSource;

impl ObservationSource for SyntheticSource {
    fn read_engine_speed(&mut self) -> Scalar {
        Scalar::new(1726.0, Unit::RPM, None)
    }
}

fn observe(source: &mut impl ObservationSource, permit: &Permit, now: Instant) -> Option<Scalar> {
    permit.claim(now).then(|| source.read_engine_speed())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::PreviewSession;
    use std::time::Duration;

    #[test]
    fn worker_returns_correlated_synthetic_sample_and_shuts_down() {
        let (worker, thread) = DemoWorker::spawn().expect("worker");
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(Instant::now()).expect("ticket");
        let permit = session.permit(ticket).expect("permit");
        worker.submit(permit).expect("submit");
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
        let permit = session.permit(ticket).expect("permit");
        assert!(worker.submit(permit.clone()).is_ok());
        assert!(matches!(
            worker.submit(permit.clone()),
            Err(TrySendError::Full(_))
        ));
        assert!(matches!(worker.receive(), Err(TryRecvError::Empty)));
        drop(requests);
        drop(replies);
        assert!(matches!(
            worker.submit(permit),
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
        let permit = session.permit(ticket).expect("permit");
        worker.submit(permit).expect("first submit");
        // Wait for the first result, then leave a distinct reply unconsumed.
        let reply = worker
            .completions
            .recv_timeout(Duration::from_secs(10))
            .expect("first reply");
        assert!(session.complete(reply.ticket, reply.sample, Instant::now()));
        let next = session.request(Instant::now()).expect("next ticket");
        let permit = session.permit(next).expect("next permit");
        worker.submit(permit).expect("second submit");
        drop(worker);
        thread.join().expect("shutdown");
    }

    #[test]
    fn revoked_and_expired_permits_are_not_processed() {
        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(now).expect("ticket");
        let permit = session.permit(ticket).expect("permit");
        let mut source = SyntheticSource;
        assert!(observe(&mut source, &permit, now + Duration::from_secs(2)).is_none());
        session.stop();
        assert!(observe(&mut source, &permit, now).is_none());
    }

    #[test]
    fn observation_boundary_reads_only_once_and_preserves_missing_data() {
        #[derive(Default)]
        struct MissingSource {
            reads: usize,
        }

        impl ObservationSource for MissingSource {
            fn read_engine_speed(&mut self) -> Scalar {
                self.reads += 1;
                Scalar::no_data()
            }
        }

        let now = Instant::now();
        let mut session = PreviewSession::default();
        assert!(session.start());
        let ticket = session.request(now).expect("ticket");
        let permit = session.permit(ticket).expect("permit");
        let mut source = MissingSource::default();
        let sample = observe(&mut source, &permit, now).expect("observation");
        assert!(session.complete(ticket, sample, now));
        assert_eq!(session.rpm_text(), "Unavailable");
        assert!(observe(&mut source, &permit, now).is_none());
        assert_eq!(source.reads, 1);
    }
}
