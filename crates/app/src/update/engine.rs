//! The update worker's logic, with nothing it cannot be given: the network, the
//! clock, the fight rule and the place it publishes to are all passed in, so
//! the tests drive it with canned answers and a clock that only moves when it
//! sleeps. One thread runs it; everything blocking (HTTP, disk, sleeping)
//! happens there, never on the UI thread or the meter's.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ed25519_dalek::VerifyingKey;

use super::http::{Http, Response};
use super::release::{self, Latest, Release};
use super::repo::Repo;
use super::schedule::Schedule;
use super::package::MAX_ARCHIVE_BYTES;
use super::stage::{self, Layout};
use super::state::{Failure, ReleaseInfo, Snapshot, UpdateState};
use super::throttle::{Pacer, MAX_BYTES_PER_SECOND};
use super::verify;
use crate::changelog::Version;

const RELEASE_ANSWER_LIMIT: usize = 1024 * 1024;
const SMALL_ASSET_LIMIT: usize = 4096;
/// Looked at least this often, so a PC that slept checks soon after waking.
const IDLE_POLL: Duration = Duration::from_secs(30);
const PAUSE_POLL: Duration = Duration::from_millis(250);
const READ_CHUNK: usize = 64 * 1024;
const MAX_RECONNECTS: u32 = 3;
/// A release's answer and the checksum and signature files are a few
/// kilobytes; one that takes longer than this has stalled.
const SMALL_BODY_TIMEOUT: Duration = Duration::from_secs(30);
const ARCHIVE_STALL_MARGIN: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum Command {
    CheckNow,
    StartDownload,
    Cancel,
    AutoDownload(bool),
    Shutdown,
}

pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
    fn sleep(&self, duration: Duration);
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

pub struct Parts {
    pub repo: Repo,
    pub running: Version,
    pub key: VerifyingKey,
    pub layout: Layout,
    pub http: Arc<dyn Http>,
    pub clock: Arc<dyn Clock>,
    /// The core's fight rule: true while the player is in a fight.
    pub in_fight: Box<dyn Fn() -> bool + Send>,
    pub publish: Box<dyn FnMut(Snapshot) + Send>,
    pub commands: Receiver<Command>,
    pub auto_download: bool,
}

enum Interrupt {
    Cancel,
    Shutdown,
    Failed(Failure),
}

enum Answer {
    Latest(Latest),
    Unreachable(String),
    RateLimited(Option<SystemTime>),
    Unusable(String),
}

pub struct Engine {
    parts: Parts,
    schedule: Schedule,
    state: UpdateState,
    last_checked: Option<SystemTime>,
    available: Option<Release>,
    staged: Option<Version>,
    last_failure: Option<(Version, Failure)>,
    /// The version whose download the player cancelled: automatic download
    /// leaves it alone until the player asks or a newer release comes.
    declined: Option<Version>,
    shutting_down: bool,
}

impl Engine {
    pub fn new(parts: Parts) -> Self {
        let now = parts.clock.now();
        Self {
            schedule: Schedule::at_launch(now),
            state: UpdateState::Checking,
            last_checked: None,
            available: None,
            staged: None,
            last_failure: None,
            declined: None,
            shutting_down: false,
            parts,
        }
    }

    pub fn run(mut self) {
        self.startup();
        while !self.shutting_down {
            let wait = self.schedule.wait(self.parts.clock.now(), IDLE_POLL);
            match self.parts.commands.recv_timeout(wait) {
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.check_if_due();
        }
    }

    /// Tidies what an earlier run left and picks up an update already staged.
    pub fn startup(&mut self) {
        stage::remove_leftovers(self.parts.layout.install_dir());
        self.staged = stage::staged_version(&self.parts.layout, self.parts.running, &self.parts.key);
        if let Some(version) = self.staged {
            self.state = UpdateState::Ready { version };
        }
        self.publish();
    }

    pub fn check_if_due(&mut self) {
        self.survive_panics(|engine| {
            if engine.schedule.automatic_due(engine.parts.clock.now()) {
                engine.check();
            }
        });
    }

    pub fn handle(&mut self, command: Command) {
        self.survive_panics(|engine| engine.dispatch(command));
    }

    /// A panic in one step (the panic hook has already logged it) must not end
    /// the worker: the window would show "Checking…" for good and no hourly
    /// check would follow. The step's state becomes a failure the window shows;
    /// a known update stays as it was, and the schedule goes on.
    fn survive_panics(&mut self, step: impl FnOnce(&mut Self)) {
        let Err(payload) = catch_unwind(AssertUnwindSafe(|| step(self))) else {
            return;
        };
        let reason = payload
            .downcast_ref::<&str>()
            .map(|message| (*message).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown cause".to_owned());
        let failure = match self.state {
            UpdateState::Checking => Some(Failure::Check(format!("the update worker stopped on an internal error: {reason}"))),
            UpdateState::Downloading { .. } => Some(Failure::Download(format!("the update worker stopped on an internal error: {reason}"))),
            _ => None,
        };
        crate::app_log::error(format!("the update worker survived an internal error: {reason}"));
        if let Some(failure) = failure {
            self.state = UpdateState::Failed { failure };
        }
        self.publish();
    }

    fn dispatch(&mut self, command: Command) {
        match command {
            Command::CheckNow => {
                if self.schedule.can_check_now(self.parts.clock.now()) {
                    self.check();
                }
            }
            Command::StartDownload => {
                if matches!(self.state, UpdateState::Available { .. }) {
                    self.declined = None;
                    self.download();
                }
            }
            Command::Cancel => {}
            Command::AutoDownload(on) => {
                self.parts.auto_download = on;
                if on {
                    self.declined = None;
                    if matches!(self.state, UpdateState::Available { .. }) {
                        self.download();
                    }
                }
            }
            Command::Shutdown => self.shutting_down = true,
        }
    }

    fn update_is_known(&self) -> bool {
        matches!(self.state, UpdateState::Available { .. } | UpdateState::Ready { .. })
    }

    fn check(&mut self) {
        let now = self.parts.clock.now();
        self.schedule.attempted(now);
        if !self.update_is_known() {
            self.state = UpdateState::Checking;
            self.publish();
        }
        let answer = self.ask_github(now);
        self.apply(answer, now);
        self.publish();
        if self.auto_download_wanted() {
            self.download();
        }
    }

    fn auto_download_wanted(&self) -> bool {
        match &self.state {
            UpdateState::Available { release, .. } => self.parts.auto_download && self.declined != Some(release.version),
            _ => false,
        }
    }

    fn ask_github(&self, now: SystemTime) -> Answer {
        let url = self.parts.repo.latest_release_url();
        let response = match self.parts.http.get(&url, "application/vnd.github+json", None, SMALL_BODY_TIMEOUT) {
            Ok(response) => response,
            Err(error) => return Answer::Unreachable(error.to_string()),
        };
        match response.status {
            200 => match response.read_limited(RELEASE_ANSWER_LIMIT) {
                Ok(body) => match release::read_latest(&body, self.parts.running, &self.parts.repo) {
                    Ok(latest) => Answer::Latest(latest),
                    Err(error) => Answer::Unusable(error),
                },
                Err(error) => Answer::Unreachable(error),
            },
            403 | 429 => Answer::RateLimited(rate_limit_until(&response, now)),
            404 => Answer::Latest(Latest::NothingNewer),
            500..=599 => Answer::Unreachable(format!("GitHub answered {}", response.status)),
            status => Answer::Unusable(format!("GitHub answered {status}")),
        }
    }

    fn apply(&mut self, answer: Answer, now: SystemTime) {
        let known = self.update_is_known();
        match answer {
            Answer::Latest(latest) => {
                self.last_checked = Some(now);
                match latest {
                    Latest::Newer(release) => self.found(release),
                    Latest::NothingNewer => {
                        self.available = None;
                        self.state = match self.staged {
                            Some(version) => UpdateState::Ready { version },
                            None => UpdateState::UpToDate { checked_at: now },
                        };
                    }
                }
            }
            Answer::Unreachable(detail) => {
                crate::app_log::warn(format!("update check: GitHub cannot be reached: {detail}"));
                if !known {
                    self.state = UpdateState::Unreachable { next_try: self.schedule.next_check_at() };
                }
            }
            Answer::RateLimited(until) => {
                self.last_checked = Some(now);
                let until = self.schedule.hold_until(now, until);
                crate::app_log::warn("update check: GitHub asked to wait before the next request");
                if !known {
                    self.state = UpdateState::RateLimited { until };
                }
            }
            Answer::Unusable(detail) => {
                self.last_checked = Some(now);
                let failure = Failure::Check(detail);
                crate::app_log::warn(failure.to_string());
                if !known {
                    self.state = UpdateState::Failed { failure };
                }
            }
        }
    }

    fn found(&mut self, release: Release) {
        let version = release.info.version;
        if self.staged.is_some_and(|staged| staged >= version) {
            self.state = UpdateState::Ready { version: self.staged.unwrap_or(version) };
            return;
        }
        if self.declined.is_some_and(|declined| declined != version) {
            self.declined = None;
        }
        let last_failure = self.last_failure.as_ref().filter(|(failed, _)| *failed == version).map(|(_, failure)| failure.clone());
        self.state = UpdateState::Available { release: release.info.clone(), last_failure };
        self.available = Some(release);
    }

    fn download(&mut self) {
        let Some(release) = self.available.clone() else {
            return;
        };
        let info = release.info.clone();
        let version = info.version;
        let Some(checksum_url) = release.checksum_url.clone() else {
            return self.download_failed(&info, Failure::MissingChecksum);
        };
        let Some(signature_url) = release.signature_url.clone() else {
            return self.download_failed(&info, Failure::MissingSignature);
        };
        if info.size > MAX_ARCHIVE_BYTES {
            return self.download_failed(&info, Failure::Download(format!("the archive is {} bytes, more than the {MAX_ARCHIVE_BYTES} an update may have", info.size)));
        }

        if let Err(error) = stage::begin_download(&self.parts.layout) {
            return self.download_failed(&info, Failure::Stage(format!("cannot prepare the staging folder: {error}")));
        }
        self.staged = None;
        self.state = UpdateState::Downloading { version, downloaded: 0, total: info.size, paused_for_fight: false };
        self.publish();

        let outcome = self.fetch_everything(&release, &checksum_url, &signature_url);
        let (checksum, signature) = match outcome {
            Ok(texts) => texts,
            Err(Interrupt::Cancel) => {
                stage::discard_download(&self.parts.layout);
                self.declined = Some(version);
                self.state = UpdateState::Available { release: info, last_failure: None };
                self.publish();
                return;
            }
            Err(Interrupt::Shutdown) => {
                stage::discard_download(&self.parts.layout);
                self.shutting_down = true;
                return;
            }
            Err(Interrupt::Failed(failure)) => {
                stage::discard_download(&self.parts.layout);
                return self.download_failed(&info, failure);
            }
        };

        self.state = UpdateState::Verifying { version };
        self.publish();
        let staged = fs::read(self.parts.layout.archive_part())
            .map_err(|error| Failure::Stage(format!("cannot read the download: {error}")))
            .and_then(|archive| {
                verify::verify_archive(&archive, version, &checksum, &signature, &self.parts.key)?;
                stage::stage(&self.parts.layout, &archive, &checksum, &signature, version)
            });
        match staged {
            Ok(()) => {
                crate::app_log::info(format!("update {version} is downloaded, verified and staged"));
                self.staged = Some(version);
                self.last_failure = None;
                self.state = UpdateState::Ready { version };
                self.publish();
            }
            Err(failure) => {
                if let Err(error) = stage::remove_all(&self.parts.layout.root()) {
                    crate::app_log::warn(format!("cannot delete the staging folder: {error}"));
                }
                self.download_failed(&info, failure);
            }
        }
    }

    fn download_failed(&mut self, release: &ReleaseInfo, failure: Failure) {
        crate::app_log::warn(format!("update {}: {failure}", release.version));
        self.last_failure = Some((release.version, failure.clone()));
        self.state = UpdateState::Available { release: release.clone(), last_failure: Some(failure) };
        self.publish();
    }

    fn fetch_everything(&mut self, release: &Release, checksum_url: &str, signature_url: &str) -> Result<(String, String), Interrupt> {
        let checksum = self.fetch_text(checksum_url, Failure::MissingChecksum)?;
        let signature = self.fetch_text(signature_url, Failure::MissingSignature)?;
        self.fetch_archive(release)?;
        Ok((checksum, signature))
    }

    fn fetch_text(&self, url: &str, unusable: Failure) -> Result<String, Interrupt> {
        let response = self
            .parts
            .http
            .get(url, "application/octet-stream", None, SMALL_BODY_TIMEOUT)
            .map_err(|error| Interrupt::Failed(Failure::Download(error.to_string())))?;
        if response.status != 200 {
            return Err(Interrupt::Failed(Failure::Download(format!("GitHub answered {}", response.status))));
        }
        let bytes = response.read_limited(SMALL_ASSET_LIMIT).map_err(|_| Interrupt::Failed(unusable.clone()))?;
        String::from_utf8(bytes).map_err(|_| Interrupt::Failed(unusable))
    }

    /// Streams the archive into `archive.zip.part`: never faster than
    /// [`MAX_BYTES_PER_SECOND`], not at all while the player fights (no
    /// connection is held then, and a Range request opens one again when the
    /// fight ends), and given up when the player cancels or turns automatic
    /// download off, which is seen after at most the read in progress: a read
    /// that stalls fails at [`archive_body_timeout`].
    fn fetch_archive(&mut self, release: &Release) -> Result<(), Interrupt> {
        let (version, total) = (release.info.version, release.info.size);
        let part = self.parts.layout.archive_part();
        let mut file = File::create(&part).map_err(|error| download_error(format!("cannot write {}: {error}", part.display())))?;

        let mut body: Option<Box<dyn Read + Send>> = None;
        let mut pacer = Pacer::new(MAX_BYTES_PER_SECOND, self.parts.clock.now());
        let mut buffer = vec![0u8; READ_CHUNK];
        let mut downloaded = 0u64;
        let mut reconnects = 0;
        let mut shown = Progress::start(total);

        while downloaded < total {
            self.poll_commands()?;
            if (self.parts.in_fight)() {
                body = None;
                self.show_progress(&mut shown, version, downloaded, total, true);
                self.parts.clock.sleep(PAUSE_POLL);
                continue;
            }
            self.show_progress(&mut shown, version, downloaded, total, false);

            let mut stream = match body.take() {
                Some(stream) => stream,
                None => self.open_archive(release, (downloaded > 0).then_some(downloaded), total - downloaded)?.body,
            };
            let read = match stream.read(&mut buffer) {
                Ok(0) => Err("the connection ended before the whole archive arrived".to_owned()),
                Ok(read) => Ok(read),
                Err(error) => Err(error.to_string()),
            };
            match read {
                Ok(read) => {
                    if read as u64 > total - downloaded {
                        return Err(download_error("the server sent more bytes than the release says the archive has".to_owned()));
                    }
                    file.write_all(&buffer[..read])
                        .map_err(|error| download_error(format!("cannot write {}: {error}", part.display())))?;
                    downloaded += read as u64;
                    body = Some(stream);
                    let wait = pacer.after_read(read, self.parts.clock.now());
                    self.parts.clock.sleep(wait);
                }
                Err(reason) => {
                    reconnects += 1;
                    if reconnects > MAX_RECONNECTS {
                        return Err(download_error(reason));
                    }
                    crate::app_log::warn(format!("update download: {reason}; asking again from byte {downloaded}"));
                }
            }
        }
        file.flush().map_err(|error| download_error(format!("cannot write {}: {error}", part.display())))?;
        drop(file);
        if body.as_mut().is_some_and(|stream| stream.read(&mut [0u8; 1]).is_ok_and(|more| more > 0)) {
            return Err(download_error("the server sent more bytes than the release says the archive has".to_owned()));
        }
        self.show_progress(&mut shown, version, downloaded, total, false);
        Ok(())
    }

    fn open_archive(&self, release: &Release, from: Option<u64>, remaining: u64) -> Result<Response, Interrupt> {
        let response = self
            .parts
            .http
            .get(&release.archive_url, "application/octet-stream", from, archive_body_timeout(remaining))
            .map_err(|error| download_error(error.to_string()))?;
        let expected = if from.is_some() { 206 } else { 200 };
        if response.status != expected {
            return Err(download_error(format!("GitHub answered {}, not {expected}", response.status)));
        }
        Ok(response)
    }

    fn poll_commands(&mut self) -> Result<(), Interrupt> {
        while let Ok(command) = self.parts.commands.try_recv() {
            match command {
                Command::Cancel => return Err(Interrupt::Cancel),
                Command::AutoDownload(on) => {
                    self.parts.auto_download = on;
                    if !on {
                        return Err(Interrupt::Cancel);
                    }
                }
                Command::Shutdown => return Err(Interrupt::Shutdown),
                Command::CheckNow | Command::StartDownload => {}
            }
        }
        Ok(())
    }

    fn show_progress(&mut self, shown: &mut Progress, version: Version, downloaded: u64, total: u64, paused: bool) {
        let step = Progress { percent: percent(downloaded, total), paused };
        if *shown != step {
            *shown = step;
            self.state = UpdateState::Downloading { version, downloaded, total, paused_for_fight: paused };
            self.publish();
        }
    }

    fn publish(&mut self) {
        let snapshot = Snapshot {
            state: self.state.clone(),
            last_checked: self.last_checked,
            check_again_at: self.schedule.check_now_available_at(),
        };
        (self.parts.publish)(snapshot);
    }
}

/// What the window last showed of a download, so it is told again only when
/// the percentage or the pause changes.
#[derive(PartialEq, Eq)]
struct Progress {
    percent: u8,
    paused: bool,
}

impl Progress {
    fn start(total: u64) -> Self {
        Self { percent: percent(0, total), paused: false }
    }
}

fn percent(downloaded: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    (downloaded.min(total) * 100 / total) as u8
}

fn download_error(detail: String) -> Interrupt {
    Interrupt::Failed(Failure::Download(detail))
}

/// What a connection asked for `remaining` bytes of the archive may take to
/// deliver them: the time the throttle itself needs, and a generous margin
/// for a link slower than the throttle. A body that takes longer has
/// stalled (a half-open connection never fails by itself): the read errors
/// and the download resumes from where it stopped.
fn archive_body_timeout(remaining: u64) -> Duration {
    Duration::from_secs(remaining.div_ceil(MAX_BYTES_PER_SECOND)) + ARCHIVE_STALL_MARGIN
}

/// `retry-after` (seconds) when GitHub sends one, else `x-ratelimit-reset`
/// (epoch seconds).
fn rate_limit_until(response: &Response, now: SystemTime) -> Option<SystemTime> {
    let seconds = |name: &str| response.header(name)?.trim().parse::<u64>().ok();
    seconds("retry-after")
        .map(|wait| now + Duration::from_secs(wait))
        .or_else(|| seconds("x-ratelimit-reset").map(|reset| UNIX_EPOCH + Duration::from_secs(reset)))
}

#[cfg(test)]
mod tests;
