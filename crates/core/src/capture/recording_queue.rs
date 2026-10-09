//! Recording on a thread of its own.
//!
//! The dispatcher must never wait for the disk: a slow disk, a rotation or a
//! prune of the recordings folder would stall it, the capture queue would fill
//! and live game packets would be lost. So the dispatcher only offers each
//! packet to a [`RecordingQueue`], which hands it to a writer thread through a
//! bounded queue (the shape of `background_save.rs`).
//!
//! When the queue is full the packet being offered is dropped whole and
//! counted. Nothing is ever cut in half, so the files stay valid recordings.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::capture::capturer::CapturedPacket;

/// Packets waiting for one writer. Game traffic peaks near 150 packets/s and
/// the worst burst of unfiltered TCP in the recordings is 3,900 packets/s, so
/// this holds about 55 s of game traffic or 2 s of that burst. Memory: 6.5 MB at
/// the 800-byte average payload seen, 72 MB if every packet were the largest
/// game payload seen (8.8 KB).
pub const RECORDING_QUEUE_CAPACITY: usize = 8_192;

/// Where a queued packet ends up. `wants` runs on the dispatcher and must never
/// wait on the disk; `write` and `flush` run on the writer thread (`flush` also
/// on the thread that asks for it).
pub trait RecordingSink: Send + Sync + 'static {
    fn wants(&self, packet: &CapturedPacket) -> bool;
    fn write(&self, packet: &CapturedPacket);
    fn flush(&self);
}

pub struct RecordingQueue {
    shared: Arc<Shared>,
    writer: Mutex<Option<JoinHandle<()>>>,
}

struct Shared {
    sink: Arc<dyn RecordingSink>,
    capacity: usize,
    state: Mutex<State>,
    work: Condvar,
    idle: Condvar,
    dropped: AtomicU64,
    /// Set when a flush gave up on the writer, cleared when the writer next
    /// finishes a write. While it is set, waiting again cannot help.
    stalled: AtomicBool,
}

#[derive(Default)]
struct State {
    queue: VecDeque<CapturedPacket>,
    writing: bool,
    shutdown: bool,
}

impl RecordingQueue {
    pub fn new(sink: Arc<dyn RecordingSink>, capacity: usize) -> Self {
        let shared = Arc::new(Shared {
            sink,
            capacity,
            state: Mutex::new(State::default()),
            work: Condvar::new(),
            idle: Condvar::new(),
            dropped: AtomicU64::new(0),
            stalled: AtomicBool::new(false),
        });
        let writer = thread::Builder::new()
            .name("recording-writer".to_string())
            .spawn({
                let shared = Arc::clone(&shared);
                move || shared.write_until_shutdown()
            })
            .expect("spawn the recording writer thread");
        Self { shared, writer: Mutex::new(Some(writer)) }
    }

    /// Queues a copy of `packet` when the sink wants it. Never waits on the
    /// disk; a full queue drops the packet and counts it.
    pub fn offer(&self, packet: &CapturedPacket) {
        if !self.shared.sink.wants(packet) {
            return;
        }
        let mut state = self.shared.state.lock().unwrap();
        if state.queue.len() >= self.shared.capacity {
            drop(state);
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        state.queue.push_back(packet.clone());
        self.shared.work.notify_one();
    }

    /// Packets dropped because the queue was full, since the queue was created.
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    /// Waits until everything offered so far has been handed to the sink or
    /// `deadline` passes, then flushes the sink. Returns `false` when the
    /// writer was still busy, without flushing the sink (a writer stuck in the
    /// sink holds what a flush would need); what it had not written yet is
    /// written later regardless. A writer already known to be stuck makes this
    /// return at once, so several flushes cost one deadline.
    pub fn flush(&self, deadline: Instant) -> bool {
        let drained = self.shared.wait_until_idle(deadline);
        if drained {
            self.shared.sink.flush();
        }
        drained
    }
}

impl Drop for RecordingQueue {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().shutdown = true;
        self.shared.work.notify_all();
        if let Some(writer) = self.writer.lock().unwrap().take() {
            let _ = writer.join();
        }
    }
}

impl Shared {
    fn write_until_shutdown(&self) {
        let mut state = self.state.lock().unwrap();
        loop {
            if let Some(packet) = state.queue.pop_front() {
                state.writing = true;
                drop(state);
                self.sink.write(&packet);
                self.stalled.store(false, Ordering::Relaxed);
                state = self.state.lock().unwrap();
                state.writing = false;
                if state.queue.is_empty() {
                    self.idle.notify_all();
                }
            } else if state.shutdown {
                return;
            } else {
                state = self.work.wait(state).unwrap();
            }
        }
    }

    fn wait_until_idle(&self, deadline: Instant) -> bool {
        let mut state = self.state.lock().unwrap();
        while !state.queue.is_empty() || state.writing {
            if self.stalled.load(Ordering::Relaxed) {
                return false;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                self.stalled.store(true, Ordering::Relaxed);
                return false;
            };
            state = self.idle.wait_timeout(state, remaining).unwrap().0;
        }
        true
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    use super::*;

    /// Holds every write until `release`, standing in for a stalled disk, and
    /// remembers the sequence numbers written.
    pub(crate) struct GatedSink {
        blocked: Mutex<bool>,
        unblocked: Condvar,
        pub(crate) writes_started: AtomicUsize,
        pub(crate) written: Mutex<Vec<u32>>,
        pub(crate) flushes: AtomicUsize,
    }

    impl GatedSink {
        pub(crate) fn blocked() -> Arc<Self> {
            Arc::new(Self {
                blocked: Mutex::new(true),
                unblocked: Condvar::new(),
                writes_started: AtomicUsize::new(0),
                written: Mutex::new(Vec::new()),
                flushes: AtomicUsize::new(0),
            })
        }

        pub(crate) fn release(&self) {
            *self.blocked.lock().unwrap() = false;
            self.unblocked.notify_all();
        }

        pub(crate) fn wait_for_write_to_start(&self) {
            let deadline = Instant::now() + Duration::from_secs(5);
            while self.writes_started.load(Ordering::SeqCst) == 0 {
                assert!(Instant::now() < deadline, "the writer never reached the sink");
                thread::sleep(Duration::from_millis(2));
            }
        }
    }

    impl RecordingSink for GatedSink {
        fn wants(&self, _packet: &CapturedPacket) -> bool {
            true
        }

        fn write(&self, packet: &CapturedPacket) {
            self.writes_started.fetch_add(1, Ordering::SeqCst);
            let mut blocked = self.blocked.lock().unwrap();
            while *blocked {
                blocked = self.unblocked.wait(blocked).unwrap();
            }
            self.written.lock().unwrap().push(packet.sequence);
        }

        fn flush(&self) {
            self.flushes.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub(crate) fn packet(sequence: u32) -> CapturedPacket {
        CapturedPacket {
            src_ip: [10, 0, 0, 1],
            src_port: 13328,
            dst_ip: [192, 168, 1, 2],
            dst_port: 51000,
            sequence,
            data: sequence.to_le_bytes().to_vec(),
            captured_at: 1_700_000_000.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use super::test_support::{packet, GatedSink};
    use super::*;
    use crate::capture::recorder::{replay_recording, PacketRecorder};

    const WAIT: Duration = Duration::from_secs(5);

    fn within(wait: Duration) -> Instant {
        Instant::now() + wait
    }

    /// A flush right after `release` can still see the writer as stuck.
    fn flushes_once_the_writer_moves(queue: &RecordingQueue) -> bool {
        let give_up = within(WAIT);
        while Instant::now() < give_up {
            if queue.flush(within(Duration::from_millis(50))) {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("belisk-recording-queue-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_stalled_sink_does_not_block_the_offering_thread() {
        let sink = GatedSink::blocked();
        let queue = RecordingQueue::new(sink.clone(), 4);

        queue.offer(&packet(0));
        sink.wait_for_write_to_start();
        let started = Instant::now();
        for sequence in 1..=100 {
            queue.offer(&packet(sequence));
        }

        assert!(started.elapsed() < Duration::from_secs(1), "offering waited on the sink");
        sink.release();
        assert!(queue.flush(within(WAIT)));
    }

    #[test]
    fn a_full_queue_drops_whole_packets_and_counts_them() {
        let sink = GatedSink::blocked();
        let queue = RecordingQueue::new(sink.clone(), 4);

        queue.offer(&packet(0));
        sink.wait_for_write_to_start();
        for sequence in 1..=10 {
            queue.offer(&packet(sequence));
        }

        assert_eq!(queue.dropped(), 6);
        sink.release();
        assert!(queue.flush(within(WAIT)));
        assert_eq!(*sink.written.lock().unwrap(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_recording_that_overflowed_is_still_a_valid_recording() {
        let dir = temp_dir("overflow");
        let recorder = Arc::new(PacketRecorder::new());
        let path = recorder.start(&dir).expect("start");
        let gate = GatedSink::blocked();
        let queue = RecordingQueue::new(Arc::new(GatedRecorder { gate: gate.clone(), recorder: recorder.clone() }), 4);

        queue.offer(&packet(0));
        gate.wait_for_write_to_start();
        for sequence in 1..=10 {
            queue.offer(&packet(sequence));
        }
        gate.release();
        assert!(queue.flush(within(WAIT)));
        recorder.stop().expect("stop");

        let mut sequences = Vec::new();
        let delivered = replay_recording(&path, |packet| {
            sequences.push(packet.sequence);
            assert_eq!(packet.data, packet.sequence.to_le_bytes().to_vec());
            true
        })
        .expect("the recording reads back");
        assert_eq!(queue.dropped(), 6);
        assert_eq!(delivered, 5);
        assert_eq!(sequences, vec![0, 1, 2, 3, 4]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn flush_writes_what_was_offered_and_flushes_the_sink() {
        let sink = GatedSink::blocked();
        sink.release();
        let queue = RecordingQueue::new(sink.clone(), 64);

        for sequence in 0..20 {
            queue.offer(&packet(sequence));
        }

        assert!(queue.flush(within(WAIT)));
        assert_eq!(*sink.written.lock().unwrap(), (0..20).collect::<Vec<u32>>());
        assert_eq!(sink.flushes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_stalled_sink_makes_flush_return_within_its_deadline_without_touching_the_sink() {
        let sink = GatedSink::blocked();
        let queue = RecordingQueue::new(sink.clone(), 4);
        queue.offer(&packet(0));
        sink.wait_for_write_to_start();

        let started = Instant::now();
        let drained = queue.flush(within(Duration::from_millis(100)));

        assert!(!drained);
        assert!(started.elapsed() < Duration::from_secs(2), "flush waited {:?}", started.elapsed());
        assert_eq!(sink.flushes.load(Ordering::SeqCst), 0, "a stuck writer holds what the sink flush needs");
        sink.release();
        assert!(flushes_once_the_writer_moves(&queue));
    }

    #[test]
    fn flushes_after_the_first_give_up_do_not_wait_again() {
        let sink = GatedSink::blocked();
        let queue = RecordingQueue::new(sink.clone(), 4);
        queue.offer(&packet(0));
        sink.wait_for_write_to_start();
        assert!(!queue.flush(within(Duration::from_millis(50))));

        let started = Instant::now();
        let later_flushes: Vec<bool> = (0..5).map(|_| queue.flush(within(WAIT))).collect();

        assert_eq!(later_flushes, vec![false; 5]);
        assert!(started.elapsed() < Duration::from_secs(1), "waited {:?}", started.elapsed());
        sink.release();
        assert!(flushes_once_the_writer_moves(&queue), "the stall ends once the writer moves again");
    }

    #[test]
    fn two_stalled_queues_flushed_under_one_deadline_cost_one_wait() {
        let (first_sink, second_sink) = (GatedSink::blocked(), GatedSink::blocked());
        let first = RecordingQueue::new(first_sink.clone(), 4);
        let second = RecordingQueue::new(second_sink.clone(), 4);
        first.offer(&packet(0));
        second.offer(&packet(0));
        first_sink.wait_for_write_to_start();
        second_sink.wait_for_write_to_start();

        let started = Instant::now();
        let deadline = within(Duration::from_millis(300));
        let results = (first.flush(deadline), second.flush(deadline));

        assert_eq!(results, (false, false));
        assert!(started.elapsed() < Duration::from_millis(1_500), "waited {:?}", started.elapsed());
        first_sink.release();
        second_sink.release();
        assert!(flushes_once_the_writer_moves(&first));
        assert!(flushes_once_the_writer_moves(&second));
    }

    /// A real recorder behind the same stall.
    struct GatedRecorder {
        gate: Arc<GatedSink>,
        recorder: Arc<PacketRecorder>,
    }

    impl RecordingSink for GatedRecorder {
        fn wants(&self, packet: &CapturedPacket) -> bool {
            self.recorder.wants(packet)
        }

        fn write(&self, packet: &CapturedPacket) {
            self.gate.write(packet);
            self.recorder.write(packet);
        }

        fn flush(&self) {
            self.recorder.flush();
        }
    }
}
