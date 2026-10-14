use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

use parking_lot::Mutex;

use super::*;
use crate::update::http::HttpError;
use crate::update::release::ARCHIVE_SUFFIX;
use crate::update::package::tests::zip_of;
use crate::update::package::EXE_ENTRY;
use crate::update::stage::tests::TempDir;
use crate::update::verify::tests::{fixture_key, sign};

const LAUNCH: u64 = 1_700_000_000;
const API: &str = "https://api.github.com/repos/o/r/releases/latest";
const HOUR: Duration = Duration::from_secs(3600);

struct ManualClock {
    now: Mutex<SystemTime>,
}

impl ManualClock {
    fn new() -> Arc<Self> {
        Arc::new(Self { now: Mutex::new(UNIX_EPOCH + Duration::from_secs(LAUNCH)) })
    }

    fn advance(&self, by: Duration) {
        *self.now.lock() += by;
    }

    fn since_launch(&self) -> Duration {
        self.now().duration_since(UNIX_EPOCH + Duration::from_secs(LAUNCH)).unwrap()
    }
}

impl Clock for ManualClock {
    fn now(&self) -> SystemTime {
        *self.now.lock()
    }

    fn sleep(&self, duration: Duration) {
        self.advance(duration);
    }
}

enum Api {
    Latest(Version),
    Status(u16, Vec<(&'static str, String)>),
    Unreachable,
    Panics,
}

type ReadHook = Arc<dyn Fn(usize) + Send + Sync>;

struct World {
    api: Api,
    archive: Vec<u8>,
    checksum: Option<String>,
    signature: Option<String>,
    /// The first archive request breaks after this many bytes.
    break_first_request_at: Option<usize>,
    on_archive_read: Option<ReadHook>,
    archive_requests: Vec<Option<u64>>,
    api_requests: usize,
    omit_signature_asset: bool,
    api_size_override: Option<u64>,
    append_to_archive_body: Vec<u8>,
    refuse_archive_host: bool,
    archive_timeouts: Vec<Duration>,
    small_body_timeouts: Vec<Duration>,
    open_bodies: Arc<AtomicUsize>,
}

struct FakeHttp {
    world: Arc<Mutex<World>>,
}

struct Breaks {
    inner: Cursor<Vec<u8>>,
    left: usize,
}

impl Read for Breaks {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 {
            return Err(io::Error::new(io::ErrorKind::ConnectionReset, "connection reset"));
        }
        let wanted = buf.len().min(self.left);
        let read = self.inner.read(&mut buf[..wanted])?;
        self.left -= read;
        Ok(read)
    }
}

struct Hooked<R> {
    inner: R,
    total: usize,
    hook: ReadHook,
}

impl<R: Read> Read for Hooked<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.total += read;
        (self.hook)(self.total);
        Ok(read)
    }
}

/// An archive connection, counted as open until it is dropped.
struct Counted<R> {
    inner: R,
    open: Arc<AtomicUsize>,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R> Drop for Counted<R> {
    fn drop(&mut self) {
        self.open.fetch_sub(1, Ordering::SeqCst);
    }
}

fn answer(status: u16, headers: Vec<(String, String)>, body: Box<dyn Read + Send>) -> Result<Response, HttpError> {
    Ok(Response { status, headers, body })
}

fn text(body: &str) -> Result<Response, HttpError> {
    answer(200, vec![], Box::new(Cursor::new(body.as_bytes().to_vec())))
}

impl Http for FakeHttp {
    fn get(&self, url: &str, _accept: &str, range_from: Option<u64>, body_timeout: Duration) -> Result<Response, HttpError> {
        let mut world = self.world.lock();
        if url == API {
            world.api_requests += 1;
            world.small_body_timeouts.push(body_timeout);
            return match &world.api {
                Api::Unreachable => Err(HttpError("dns error".to_owned())),
                Api::Panics => panic!("the transport is broken"),
                Api::Status(status, headers) => {
                    let headers = headers.iter().map(|(name, value)| ((*name).to_owned(), value.clone())).collect();
                    answer(*status, headers, Box::new(Cursor::new(b"{}".to_vec())))
                }
                Api::Latest(version) => {
                    let size = world.api_size_override.unwrap_or(world.archive.len() as u64);
                    text(&release_json(*version, size, !world.omit_signature_asset))
                }
            };
        }
        if url.ends_with(".sha256") {
            world.small_body_timeouts.push(body_timeout);
            return match world.checksum.clone() {
                Some(text_) => text(&text_),
                None => answer(404, vec![], Box::new(Cursor::new(vec![]))),
            };
        }
        if url.ends_with(".sig") {
            world.small_body_timeouts.push(body_timeout);
            return match world.signature.clone() {
                Some(text_) => text(&text_),
                None => answer(404, vec![], Box::new(Cursor::new(vec![]))),
            };
        }
        world.archive_requests.push(range_from);
        world.archive_timeouts.push(body_timeout);
        if world.refuse_archive_host {
            return Err(HttpError(format!("{url} is not an address updates may use: that host is not one of GitHub's")));
        }
        let first = world.archive_requests.len() == 1;
        let start = range_from.unwrap_or(0) as usize;
        let mut rest = world.archive[start..].to_vec();
        rest.extend_from_slice(&world.append_to_archive_body);
        let mut body: Box<dyn Read + Send> = Box::new(Cursor::new(rest.clone()));
        if let (true, Some(at)) = (first, world.break_first_request_at) {
            body = Box::new(Breaks { inner: Cursor::new(rest), left: at });
        }
        if let Some(hook) = world.on_archive_read.clone() {
            body = Box::new(Hooked { inner: body, total: 0, hook });
        }
        world.open_bodies.fetch_add(1, Ordering::SeqCst);
        body = Box::new(Counted { inner: body, open: world.open_bodies.clone() });
        let status = if range_from.is_some() { 206 } else { 200 };
        answer(status, vec![], body)
    }
}

fn release_json(version: Version, archive_size: u64, with_signature: bool) -> String {
    let name = format!("belisk-{version}{ARCHIVE_SUFFIX}");
    let signature = if with_signature {
        format!(r#",{{"name":"{name}.sig","size":128,"browser_download_url":"https://assets.test/{name}.sig"}}"#)
    } else {
        String::new()
    };
    format!(
        r#"{{"tag_name":"v{version}","html_url":"https://github.com/o/r/releases/tag/v{version}","draft":false,"prerelease":false,
        "assets":[
          {{"name":"{name}","size":{archive_size},"browser_download_url":"https://assets.test/{name}"}},
          {{"name":"{name}.sha256","size":64,"browser_download_url":"https://assets.test/{name}.sha256"}}{signature}
        ]}}"#
    )
}

fn pseudo_random(seed: u8, len: usize) -> Vec<u8> {
    let mut state = 0x9E37_79B9_7F4A_7C15u64 ^ u64::from(seed);
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as u8
        })
        .collect()
}

const ARCHIVE_EXE_BYTES: usize = 2_500_000;

struct Rig {
    engine: Engine,
    clock: Arc<ManualClock>,
    world: Arc<Mutex<World>>,
    shown: Arc<Mutex<Vec<Snapshot>>>,
    commands: Sender<Command>,
    layout: Layout,
    fight: Arc<Mutex<Option<(Duration, Duration)>>>,
    /// How many archive connections were open at each check made during the fight.
    open_during_fight: Arc<Mutex<Vec<usize>>>,
    _dir: TempDir,
}

impl Rig {
    fn new(name: &str, auto_download: bool) -> Self {
        let dir = TempDir::new(name);
        let install = dir.0.join("install");
        std::fs::create_dir_all(&install).unwrap();
        let layout = Layout::new(install);
        let clock = ManualClock::new();
        let world = Arc::new(Mutex::new(World {
            api: Api::Latest(Version::new(1, 1, 0)),
            archive: vec![],
            checksum: None,
            signature: None,
            break_first_request_at: None,
            on_archive_read: None,
            archive_requests: vec![],
            api_requests: 0,
            omit_signature_asset: false,
            api_size_override: None,
            append_to_archive_body: vec![],
            refuse_archive_host: false,
            archive_timeouts: vec![],
            small_body_timeouts: vec![],
            open_bodies: Arc::default(),
        }));
        let shown = Arc::new(Mutex::new(Vec::new()));
        let (commands, receiver) = channel();
        let fight: Arc<Mutex<Option<(Duration, Duration)>>> = Arc::new(Mutex::new(None));
        let open_during_fight = Arc::new(Mutex::new(Vec::new()));
        let in_fight = {
            let (clock, fight, open) = (clock.clone(), fight.clone(), open_during_fight.clone());
            let bodies = world.lock().open_bodies.clone();
            move || {
                let fighting = fight.lock().is_some_and(|(from, until)| (from..until).contains(&clock.since_launch()));
                if fighting {
                    open.lock().push(bodies.load(Ordering::SeqCst));
                }
                fighting
            }
        };
        let publish = {
            let shown = shown.clone();
            move |snapshot| shown.lock().push(snapshot)
        };
        let engine = Engine::new(Parts {
            repo: "o/r".parse().unwrap(),
            running: Version::new(1, 0, 0),
            key: fixture_key().verifying_key(),
            layout: layout.clone(),
            http: Arc::new(FakeHttp { world: world.clone() }),
            clock: clock.clone(),
            in_fight: Box::new(in_fight),
            publish: Box::new(publish),
            commands: receiver,
            auto_download,
        });
        let rig = Self { engine, clock, world, shown, commands, layout, fight, open_during_fight, _dir: dir };
        rig.publish_release(Version::new(1, 1, 0));
        rig
    }

    fn publish_release(&self, version: Version) {
        let exe = pseudo_random(version.minor as u8, ARCHIVE_EXE_BYTES);
        let archive = zip_of(&[(EXE_ENTRY, &exe), ("WinDivert.dll", b"dll")]);
        let mut world = self.world.lock();
        world.checksum = Some(format!("{}  belisk-{version}{ARCHIVE_SUFFIX}\n", crate::update::verify::sha256_hex(&archive)));
        world.signature = Some(sign(&fixture_key(), version, &archive));
        world.archive = archive;
        world.api = Api::Latest(version);
    }

    fn state(&self) -> UpdateState {
        self.engine.state.clone()
    }

    fn api_requests(&self) -> usize {
        self.world.lock().api_requests
    }

    fn archive_requests(&self) -> Vec<Option<u64>> {
        self.world.lock().archive_requests.clone()
    }

    fn launch_check(&mut self) {
        self.engine.startup();
        self.engine.check_if_due();
    }

    fn after(&mut self, wait: Duration) {
        self.clock.advance(wait);
        self.engine.check_if_due();
    }

    fn on_archive_read(&self, hook: impl Fn(usize) + Send + Sync + 'static) {
        self.world.lock().on_archive_read = Some(Arc::new(hook));
    }

    fn ready_archive(&self) -> Option<Vec<u8>> {
        std::fs::read(self.layout.ready_dir().join("update.zip")).ok()
    }

    fn shown_states(&self) -> Vec<UpdateState> {
        self.shown.lock().iter().map(|snapshot| snapshot.state.clone()).collect()
    }
}

fn available(version: Version) -> impl Fn(&UpdateState) -> bool {
    move |state| matches!(state, UpdateState::Available { release, .. } if release.version == version)
}

#[test]
fn a_newer_release_found_at_launch_waits_for_the_click_when_automatic_download_is_off() {
    let mut rig = Rig::new("engine-manual", false);

    rig.launch_check();

    let UpdateState::Available { release, last_failure: None } = rig.state() else { panic!("{:?}", rig.state()) };
    assert_eq!(release.version, Version::new(1, 1, 0));
    assert_eq!(release.notes_url, "https://github.com/o/r/releases/tag/v1.1.0");
    assert_eq!(release.size, rig.world.lock().archive.len() as u64);
    assert!(rig.archive_requests().is_empty(), "nothing is downloaded without the click");

    rig.engine.handle(Command::StartDownload);

    assert_eq!(rig.state(), UpdateState::Ready { version: Version::new(1, 1, 0) });
    assert_eq!(rig.ready_archive().unwrap(), rig.world.lock().archive);
    let states = rig.shown_states();
    assert!(states.iter().any(|state| matches!(state, UpdateState::Downloading { .. })));
    assert!(states.iter().any(|state| matches!(state, UpdateState::Verifying { .. })));
}

#[test]
fn with_automatic_download_on_available_turns_into_ready_by_itself() {
    let mut rig = Rig::new("engine-auto", true);

    rig.launch_check();

    assert_eq!(rig.state(), UpdateState::Ready { version: Version::new(1, 1, 0) });
    assert!(rig.shown_states().iter().any(available(Version::new(1, 1, 0))), "it was available first");
}

#[test]
fn turning_automatic_download_on_starts_the_download_of_an_available_update() {
    let mut rig = Rig::new("engine-auto-switch", false);
    rig.launch_check();

    rig.engine.handle(Command::AutoDownload(true));

    assert_eq!(rig.state(), UpdateState::Ready { version: Version::new(1, 1, 0) });
}

#[test]
fn the_running_version_is_up_to_date() {
    let mut rig = Rig::new("engine-uptodate", true);
    rig.world.lock().api = Api::Latest(Version::new(1, 0, 0));

    rig.launch_check();

    assert_eq!(rig.state(), UpdateState::UpToDate { checked_at: UNIX_EPOCH + Duration::from_secs(LAUNCH) });
}

#[test]
fn it_checks_at_launch_then_once_an_hour_and_never_sooner() {
    let mut rig = Rig::new("engine-hourly", false);
    rig.launch_check();
    assert_eq!(rig.api_requests(), 1);

    rig.after(Duration::from_secs(30 * 60));
    assert_eq!(rig.api_requests(), 1);
    rig.after(Duration::from_secs(29 * 60 + 59));
    assert_eq!(rig.api_requests(), 1);
    rig.after(Duration::from_secs(1));
    assert_eq!(rig.api_requests(), 2);
    rig.after(HOUR - Duration::from_secs(1));
    assert_eq!(rig.api_requests(), 2);
    rig.after(Duration::from_secs(1));
    assert_eq!(rig.api_requests(), 3);
}

#[test]
fn a_pc_that_slept_for_hours_checks_once_when_it_wakes() {
    let mut rig = Rig::new("engine-wake", false);
    rig.launch_check();

    rig.after(Duration::from_secs(9 * 3600));
    rig.engine.check_if_due();

    assert_eq!(rig.api_requests(), 2, "one check on waking, not one per missed hour");
}

#[test]
fn hourly_checks_go_on_while_an_update_is_available_or_staged() {
    let mut rig = Rig::new("engine-keeps-checking", false);
    rig.launch_check();
    rig.engine.handle(Command::StartDownload);
    assert!(matches!(rig.state(), UpdateState::Ready { .. }));

    rig.after(HOUR);

    assert_eq!(rig.api_requests(), 2);
    assert_eq!(rig.state(), UpdateState::Ready { version: Version::new(1, 1, 0) }, "the staged update stays shown");
}

#[test]
fn a_newer_release_replaces_the_staged_one() {
    let mut rig = Rig::new("engine-replaces-staged", true);
    rig.launch_check();
    let first = rig.ready_archive().unwrap();

    rig.publish_release(Version::new(1, 2, 0));
    rig.after(HOUR);

    assert_eq!(rig.state(), UpdateState::Ready { version: Version::new(1, 2, 0) });
    assert_ne!(rig.ready_archive().unwrap(), first, "the files on disk are the newer release's");
    assert_eq!(stage::staged_version(&rig.layout, Version::new(1, 0, 0), &stage::tests::key()), Some(Version::new(1, 2, 0)));
}

#[test]
fn a_newer_release_with_manual_download_is_offered_while_the_old_one_stays_staged() {
    let mut rig = Rig::new("engine-newer-manual", false);
    rig.launch_check();
    rig.engine.handle(Command::StartDownload);

    rig.publish_release(Version::new(1, 2, 0));
    rig.after(HOUR);

    assert!(available(Version::new(1, 2, 0))(&rig.state()), "{:?}", rig.state());
    assert!(rig.ready_archive().is_some(), "the 1.1.0 files are still there until 1.2.0 is downloaded");
}

#[test]
fn check_now_does_nothing_within_a_minute_of_a_check_and_works_after() {
    let mut rig = Rig::new("engine-check-now", false);
    rig.launch_check();
    assert_eq!(rig.api_requests(), 1);

    rig.clock.advance(Duration::from_secs(59));
    rig.engine.handle(Command::CheckNow);
    assert_eq!(rig.api_requests(), 1);

    rig.clock.advance(Duration::from_secs(1));
    rig.engine.handle(Command::CheckNow);
    assert_eq!(rig.api_requests(), 2);
    let last = rig.shown.lock().last().cloned().unwrap();
    assert_eq!(last.check_again_at, rig.clock.now() + Duration::from_secs(60), "the window shows when it works again");
}

#[test]
fn a_rate_limit_is_waited_out_by_retry_after_and_never_looped_on() {
    let mut rig = Rig::new("engine-retry-after", false);
    rig.world.lock().api = Api::Status(429, vec![("retry-after", "7200".to_owned())]);

    rig.launch_check();

    let until = rig.clock.now() + Duration::from_secs(7200);
    assert_eq!(rig.state(), UpdateState::RateLimited { until });
    rig.after(HOUR + Duration::from_secs(1));
    assert_eq!(rig.api_requests(), 1, "the hourly time came but GitHub asked for two hours");
    rig.engine.handle(Command::CheckNow);
    assert_eq!(rig.api_requests(), 1, "Check now waits too");

    rig.after(HOUR);
    assert_eq!(rig.api_requests(), 2);
}

#[test]
fn a_403_waits_until_the_reset_time_github_gives() {
    let mut rig = Rig::new("engine-reset", false);
    let reset = LAUNCH + 5400;
    rig.world.lock().api = Api::Status(403, vec![("x-ratelimit-reset", reset.to_string()), ("x-ratelimit-remaining", "0".to_owned())]);

    rig.launch_check();

    assert_eq!(rig.state(), UpdateState::RateLimited { until: UNIX_EPOCH + Duration::from_secs(reset) });
    rig.after(HOUR);
    assert_eq!(rig.api_requests(), 1);
    rig.after(Duration::from_secs(1800));
    assert_eq!(rig.api_requests(), 2);
}

#[test]
fn a_rate_limit_leaves_an_available_update_showing() {
    let mut rig = Rig::new("engine-limit-keeps", false);
    rig.launch_check();
    rig.world.lock().api = Api::Status(429, vec![]);

    rig.after(HOUR);

    assert!(available(Version::new(1, 1, 0))(&rig.state()));
}

#[test]
fn no_network_is_unreachable_until_the_next_hourly_check_and_does_not_disturb_a_known_update() {
    let mut rig = Rig::new("engine-offline", false);
    rig.world.lock().api = Api::Unreachable;

    rig.launch_check();

    assert_eq!(rig.state(), UpdateState::Unreachable { next_try: rig.clock.now() + HOUR });
    rig.after(Duration::from_secs(1800));
    assert_eq!(rig.api_requests(), 1, "no retry loop");

    rig.world.lock().api = Api::Latest(Version::new(1, 1, 0));
    rig.after(HOUR);
    assert!(available(Version::new(1, 1, 0))(&rig.state()));

    rig.world.lock().api = Api::Unreachable;
    rig.after(HOUR);
    assert!(available(Version::new(1, 1, 0))(&rig.state()), "an update that was found stays found");
}

#[test]
fn a_panic_in_a_check_is_a_failed_check_and_the_hourly_schedule_goes_on() {
    let mut rig = Rig::new("engine-panic", false);
    rig.world.lock().api = Api::Panics;

    rig.launch_check();

    let state = rig.state();
    assert!(
        matches!(&state, UpdateState::Failed { failure: Failure::Check(reason) } if reason.contains("the transport is broken")),
        "{state:?}"
    );
    assert_eq!(rig.shown_states().last(), Some(&rig.state()), "the window is told");

    rig.world.lock().api = Api::Latest(Version::new(1, 1, 0));
    rig.after(HOUR);
    assert!(available(Version::new(1, 1, 0))(&rig.state()), "the next hourly check still runs");
}

#[test]
fn a_panic_in_a_check_leaves_a_known_update_showing() {
    let mut rig = Rig::new("engine-panic-known", false);
    rig.launch_check();
    rig.world.lock().api = Api::Panics;

    rig.after(HOUR);

    assert!(available(Version::new(1, 1, 0))(&rig.state()), "{:?}", rig.state());
}

#[test]
fn an_answer_that_is_not_a_release_is_a_failed_check_not_a_crash() {
    let mut rig = Rig::new("engine-garbage", false);
    rig.world.lock().api = Api::Status(200, vec![]);

    rig.launch_check();

    assert!(matches!(rig.state(), UpdateState::Failed { failure: Failure::Check(_) }), "{:?}", rig.state());
}

#[test]
fn a_repository_without_releases_is_up_to_date() {
    let mut rig = Rig::new("engine-404", false);
    rig.world.lock().api = Api::Status(404, vec![]);

    rig.launch_check();

    assert!(matches!(rig.state(), UpdateState::UpToDate { .. }));
}

#[test]
fn the_download_takes_at_least_its_size_over_the_cap() {
    let mut rig = Rig::new("engine-cap", true);
    let started = rig.clock.since_launch();

    rig.launch_check();

    let size = rig.world.lock().archive.len() as f64;
    let seconds = (rig.clock.since_launch() - started).as_secs_f64();
    assert!(seconds >= size / MAX_BYTES_PER_SECOND as f64 * 0.98, "{size} bytes in {seconds} s");
    assert!(matches!(rig.state(), UpdateState::Ready { .. }));
}

#[test]
fn the_download_pauses_during_a_fight_reads_nothing_and_resumes_after_it() {
    let mut rig = Rig::new("engine-fight", true);
    let pause = (Duration::from_secs(1), Duration::from_secs(61));
    *rig.fight.lock() = Some(pause);
    let reads = Arc::new(Mutex::new(Vec::<Duration>::new()));
    {
        let (clock, reads) = (rig.clock.clone(), reads.clone());
        rig.on_archive_read(move |_| reads.lock().push(clock.since_launch()));
    }

    rig.launch_check();

    assert!(matches!(rig.state(), UpdateState::Ready { .. }));
    let reads = reads.lock();
    assert!(reads.iter().all(|at| !(pause.0 + Duration::from_millis(100)..pause.1).contains(at)), "nothing is read while fighting");
    assert!(reads.iter().any(|at| *at >= pause.1), "it went on after the fight");
    let paused: Vec<_> = rig
        .shown_states()
        .into_iter()
        .filter(|state| matches!(state, UpdateState::Downloading { paused_for_fight: true, .. }))
        .collect();
    assert_eq!(paused.len(), 1, "the window is told once when it pauses");
    let resumed = rig.shown_states().iter().rposition(|state| matches!(state, UpdateState::Downloading { paused_for_fight: false, .. }));
    assert!(resumed.is_some());
}

#[test]
fn no_connection_is_held_during_a_fight_and_a_range_request_reopens_it_after() {
    let mut rig = Rig::new("engine-fight-drops-body", true);
    *rig.fight.lock() = Some((Duration::from_secs(1), Duration::from_secs(61)));

    rig.launch_check();

    assert!(matches!(rig.state(), UpdateState::Ready { .. }));
    let held = rig.open_during_fight.lock().clone();
    assert!(held.len() > 1 && held[1..].iter().all(|open| *open == 0), "connections open at each check after the first of the fight: {held:?}");
    let requests = rig.archive_requests();
    assert_eq!(requests.len(), 2, "{requests:?}");
    assert_eq!(requests[0], None);
    assert!(requests[1].is_some_and(|from| from > 0), "the second request asks for the rest: {requests:?}");
    assert_eq!(rig.ready_archive().unwrap(), rig.world.lock().archive, "the file is whole");
}

#[test]
fn a_fight_before_the_first_byte_opens_the_connection_only_after_it() {
    let mut rig = Rig::new("engine-fight-first", true);
    *rig.fight.lock() = Some((Duration::ZERO, Duration::from_secs(30)));

    rig.launch_check();

    assert_eq!(rig.archive_requests(), vec![None], "nothing was opened to wait in");
    assert!(matches!(rig.state(), UpdateState::Ready { .. }));
}

#[test]
fn each_archive_request_may_take_what_is_left_at_the_cap_plus_a_margin() {
    let mut rig = Rig::new("engine-body-timeout", true);
    rig.world.lock().break_first_request_at = Some(300_000);

    rig.launch_check();

    let size = rig.world.lock().archive.len() as u64;
    assert_eq!(
        rig.world.lock().archive_timeouts,
        vec![archive_body_timeout(size), archive_body_timeout(size - 300_000)],
    );
    assert!(archive_body_timeout(size) > Duration::from_secs(size / MAX_BYTES_PER_SECOND), "with a margin on top");
    assert!(rig.world.lock().small_body_timeouts.iter().all(|timeout| *timeout == SMALL_BODY_TIMEOUT));
}

#[test]
fn the_window_is_told_each_percent_once() {
    let mut rig = Rig::new("engine-progress", true);

    rig.launch_check();

    let percents: Vec<u64> = rig
        .shown_states()
        .iter()
        .filter_map(|state| match state {
            UpdateState::Downloading { downloaded, total, .. } => Some(downloaded * 100 / total),
            _ => None,
        })
        .collect();
    assert!(percents.windows(2).all(|pair| pair[0] < pair[1]), "{percents:?}");
    assert_eq!(percents.last(), Some(&100));
}

#[test]
fn cancel_deletes_the_partial_file_and_automatic_download_leaves_that_version_alone() {
    let mut rig = Rig::new("engine-cancel", true);
    let commands = Mutex::new(rig.commands.clone());
    rig.on_archive_read(move |total| {
        if total > 200_000 {
            let _ = commands.lock().send(Command::Cancel);
        }
    });

    rig.launch_check();

    assert!(available(Version::new(1, 1, 0))(&rig.state()), "{:?}", rig.state());
    assert!(matches!(rig.state(), UpdateState::Available { last_failure: None, .. }));
    assert!(!rig.layout.download_dir().exists(), "the partial file is gone");
    assert!(rig.ready_archive().is_none());

    rig.world.lock().on_archive_read = None;
    rig.after(HOUR);
    assert_eq!(rig.archive_requests().len(), 1, "the hourly check does not start what the player cancelled");

    rig.engine.handle(Command::StartDownload);
    assert!(matches!(rig.state(), UpdateState::Ready { .. }), "the player can still ask");
}

#[test]
fn turning_automatic_download_off_mid_download_cancels_it_and_deletes_the_partial_file() {
    let mut rig = Rig::new("engine-auto-off", true);
    let commands = Mutex::new(rig.commands.clone());
    rig.on_archive_read(move |total| {
        if total > 200_000 {
            let _ = commands.lock().send(Command::AutoDownload(false));
        }
    });

    rig.launch_check();

    assert!(available(Version::new(1, 1, 0))(&rig.state()));
    assert!(!rig.layout.download_dir().exists());
    assert!(!rig.engine.parts.auto_download);
    rig.after(HOUR);
    assert_eq!(rig.archive_requests().len(), 1, "automatic download is off now");
}

#[test]
fn a_connection_lost_mid_download_resumes_from_where_it_stopped() {
    let mut rig = Rig::new("engine-resume", true);
    rig.world.lock().break_first_request_at = Some(300_000);

    rig.launch_check();

    assert_eq!(rig.state(), UpdateState::Ready { version: Version::new(1, 1, 0) });
    assert_eq!(rig.archive_requests(), vec![None, Some(300_000)]);
    assert_eq!(rig.ready_archive().unwrap(), rig.world.lock().archive, "the resumed file verified, so it is whole");
}

#[test]
fn a_connection_that_keeps_failing_gives_up_with_the_reason_and_leaves_nothing_behind() {
    let mut rig = Rig::new("engine-gives-up", false);
    rig.launch_check();
    rig.world.lock().api = Api::Unreachable;
    rig.world.lock().archive.truncate(1000);

    rig.engine.handle(Command::StartDownload);

    assert!(matches!(rig.state(), UpdateState::Available { last_failure: Some(Failure::Download(_)), .. }), "{:?}", rig.state());
    assert!(!rig.layout.download_dir().exists() && !rig.layout.ready_dir().exists());
}

#[test]
fn a_wrong_hash_is_deleted_and_never_installed() {
    let mut rig = Rig::new("engine-bad-hash", true);
    rig.world.lock().checksum = Some(crate::update::verify::sha256_hex(b"something else"));

    rig.launch_check();

    assert!(matches!(rig.state(), UpdateState::Available { last_failure: Some(Failure::ChecksumMismatch), .. }));
    assert!(!rig.layout.root().exists(), "the whole staging folder is deleted");
}

#[test]
fn a_signature_by_another_key_is_deleted_and_never_installed() {
    let mut rig = Rig::new("engine-bad-signature", true);
    let forger = ed25519_dalek::SigningKey::from_bytes(&[0x13; 32]);
    let archive = rig.world.lock().archive.clone();
    rig.world.lock().signature = Some(sign(&forger, Version::new(1, 1, 0), &archive));

    rig.launch_check();

    assert!(matches!(rig.state(), UpdateState::Available { last_failure: Some(Failure::SignatureMismatch), .. }));
    assert!(!rig.layout.root().exists());
}

#[test]
fn a_release_without_its_signature_asset_is_not_downloaded_and_leaves_the_staged_update_alone() {
    let mut rig = Rig::new("engine-no-signature", false);
    rig.launch_check();
    rig.engine.handle(Command::StartDownload);
    assert!(matches!(rig.state(), UpdateState::Ready { .. }));

    rig.publish_release(Version::new(1, 2, 0));
    rig.world.lock().omit_signature_asset = true;
    rig.after(HOUR);
    rig.engine.handle(Command::StartDownload);

    assert!(matches!(rig.state(), UpdateState::Available { last_failure: Some(Failure::MissingSignature), .. }), "{:?}", rig.state());
    assert_eq!(rig.archive_requests().len(), 1, "nothing of 1.2.0 was fetched");
    assert!(rig.ready_archive().is_some(), "the staged 1.1.0 was not touched");
}

#[test]
fn after_a_failed_download_automatic_mode_tries_again_at_the_next_hourly_check_only() {
    let mut rig = Rig::new("engine-retry", true);
    rig.world.lock().checksum = Some(crate::update::verify::sha256_hex(b"wrong"));

    rig.launch_check();
    assert_eq!(rig.archive_requests().len(), 1, "one attempt, no loop");
    rig.after(Duration::from_secs(1800));
    assert_eq!(rig.archive_requests().len(), 1);

    rig.after(Duration::from_secs(1800));
    assert_eq!(rig.archive_requests().len(), 2, "the next hourly check tries once more");
}

#[test]
fn a_failure_is_remembered_for_the_release_it_belongs_to_only() {
    let mut rig = Rig::new("engine-failure-scope", false);
    rig.world.lock().checksum = Some(crate::update::verify::sha256_hex(b"wrong"));
    rig.launch_check();
    rig.engine.handle(Command::StartDownload);
    assert!(matches!(rig.state(), UpdateState::Available { last_failure: Some(_), .. }));

    rig.publish_release(Version::new(1, 2, 0));
    rig.after(HOUR);

    assert!(matches!(rig.state(), UpdateState::Available { last_failure: None, .. }));
}

#[test]
fn an_update_staged_by_an_earlier_run_is_ready_at_start_without_any_network() {
    let mut rig = Rig::new("engine-startup-staged", false);
    let update = stage::tests::published(&[(EXE_ENTRY, b"staged exe")], Version::new(1, 5, 0));
    stage::tests::stage_published(&rig.layout, &update, Version::new(1, 5, 0));

    rig.engine.startup();

    assert_eq!(rig.state(), UpdateState::Ready { version: Version::new(1, 5, 0) });
    assert_eq!(rig.api_requests(), 0);
}

#[test]
fn a_staged_update_changed_since_it_was_staged_is_not_ready_at_start_and_is_deleted() {
    let mut rig = Rig::new("engine-startup-tampered", false);
    let update = stage::tests::published(&[(EXE_ENTRY, b"staged exe")], Version::new(1, 5, 0));
    stage::tests::stage_published(&rig.layout, &update, Version::new(1, 5, 0));
    std::fs::write(rig.layout.ready_dir().join("update.zip"), zip_of(&[(EXE_ENTRY, b"planted")])).unwrap();

    rig.engine.startup();

    assert!(!matches!(rig.state(), UpdateState::Ready { .. }), "{:?}", rig.state());
    assert!(!rig.layout.root().exists());
}

#[test]
fn an_archive_the_release_says_is_larger_than_an_update_may_be_is_not_downloaded() {
    let mut rig = Rig::new("engine-too-big", false);
    rig.world.lock().api_size_override = Some(MAX_ARCHIVE_BYTES + 1);
    rig.launch_check();

    rig.engine.handle(Command::StartDownload);

    assert!(matches!(rig.state(), UpdateState::Available { last_failure: Some(Failure::Download(_)), .. }), "{:?}", rig.state());
    assert!(rig.archive_requests().is_empty(), "nothing was requested");
    assert!(!rig.layout.root().exists());
}

#[test]
fn a_server_that_sends_more_than_the_release_says_fails_the_download() {
    let mut rig = Rig::new("engine-too-long", true);
    rig.world.lock().append_to_archive_body = b"trailing bytes".to_vec();

    rig.launch_check();

    assert!(matches!(rig.state(), UpdateState::Available { last_failure: Some(Failure::Download(_)), .. }), "{:?}", rig.state());
    assert!(!rig.layout.download_dir().exists() && !rig.layout.ready_dir().exists());
}

#[test]
fn an_address_the_http_layer_refuses_is_a_failed_download_with_a_reason() {
    let mut rig = Rig::new("engine-refused-host", true);
    rig.world.lock().refuse_archive_host = true;

    rig.launch_check();

    let UpdateState::Available { last_failure: Some(Failure::Download(reason)), .. } = rig.state() else { panic!("{:?}", rig.state()) };
    assert!(reason.contains("not an address updates may use"), "{reason}");
}

#[test]
fn shutting_down_during_a_download_stops_it_and_deletes_the_partial_file() {
    let mut rig = Rig::new("engine-shutdown", true);
    let commands = Mutex::new(rig.commands.clone());
    rig.on_archive_read(move |total| {
        if total > 200_000 {
            let _ = commands.lock().send(Command::Shutdown);
        }
    });

    rig.launch_check();

    assert!(rig.engine.shutting_down);
    assert!(!rig.layout.download_dir().exists());
}
