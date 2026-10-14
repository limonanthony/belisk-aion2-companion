use std::io::Read;
use std::time::Duration;

/// The update code's only network access. Production uses [`UreqHttp`]; the
/// tests give it canned answers, so nothing in them touches a network.
pub trait Http: Send + Sync {
    /// `GET url`. A status of any kind is an answer, not an error; only a
    /// failure to get one (DNS, connection, TLS, timeout, an address that is not
    /// allowed) is. `range_from` asks for the bytes from that offset on.
    /// Reading the body takes at most `body_timeout` in all, counted from the
    /// headers: a connection that stalls half-open ends in a read error
    /// instead of blocking the worker for ever.
    fn get(&self, url: &str, accept: &str, range_from: Option<u64>, body_timeout: Duration) -> Result<Response, HttpError>;
}

pub struct Response {
    pub status: u16,
    /// Header names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Box<dyn Read + Send>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// The body, or an error when it is longer than `limit` bytes.
    pub fn read_limited(self, limit: usize) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        self.body
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("reading the answer failed: {error}"))?;
        if bytes.len() > limit {
            return Err(format!("the answer is longer than {limit} bytes"));
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpError(pub String);

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 5;

/// Where a request may go: the API, the release page's download address and the
/// hosts GitHub redirects asset downloads to. Every address is checked, the one
/// asked for and each redirect it leads to, and only `https` on the standard
/// port is accepted.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    https_only: bool,
    /// Only the tests' loopback server has a port; GitHub is always on the standard one.
    any_port: bool,
    hosts: &'static [&'static str],
}

impl Policy {
    pub const GITHUB: Self = Self {
        https_only: true,
        any_port: false,
        hosts: &["api.github.com", "github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com"],
    };

    pub fn check(&self, url: &str) -> Result<(), HttpError> {
        let refuse = |why: &str| Err(HttpError(format!("{url} is not an address updates may use: {why}")));
        let rest = if let Some(rest) = strip_scheme(url, "https://") {
            rest
        } else if let (false, Some(rest)) = (self.https_only, strip_scheme(url, "http://")) {
            rest
        } else {
            return refuse("it is not https");
        };
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let (host, port) = match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        };
        let odd_port = port.is_some_and(|port| !self.any_port || port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()));
        if host.is_empty() || host.contains(['@', '\\', ' ']) || odd_port {
            return refuse("it names a user, a port or nothing");
        }
        if !self.hosts.iter().any(|allowed| allowed.eq_ignore_ascii_case(host)) {
            return refuse("that host is not one of GitHub's");
        }
        Ok(())
    }
}

fn strip_scheme<'a>(url: &'a str, scheme: &str) -> Option<&'a str> {
    let head = url.get(..scheme.len())?;
    head.eq_ignore_ascii_case(scheme).then(|| &url[scheme.len()..])
}

/// Windows' own TLS (Schannel) through `ureq`, with the app's name and version
/// as the User-Agent GitHub requires. Nothing else about the player is sent.
pub struct UreqHttp {
    agent: ureq::Agent,
    policy: Policy,
}

impl UreqHttp {
    pub fn new() -> Self {
        Self::with_policy(Policy::GITHUB)
    }

    fn with_policy(policy: Policy) -> Self {
        let config = ureq::Agent::config_builder()
            .user_agent(concat!("belisk/", env!("CARGO_PKG_VERSION")))
            .http_status_as_error(false)
            .https_only(policy.https_only)
            .max_redirects(0)
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .provider(ureq::tls::TlsProvider::NativeTls)
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT))
            .build();
        Self { agent: ureq::Agent::new_with_config(config), policy }
    }
}

impl Http for UreqHttp {
    fn get(&self, url: &str, accept: &str, range_from: Option<u64>, body_timeout: Duration) -> Result<Response, HttpError> {
        let mut current = url.to_owned();
        for _ in 0..=MAX_REDIRECTS {
            self.policy.check(&current)?;
            let mut request = self.agent.get(&current).header("Accept", accept);
            if let Some(offset) = range_from {
                request = request.header("Range", format!("bytes={offset}-"));
            }
            let response = request
                .config()
                .timeout_recv_body(Some(body_timeout))
                .build()
                .call()
                .map_err(|error| HttpError(error.to_string()))?;
            let status = response.status().as_u16();
            let headers: Vec<(String, String)> = response
                .headers()
                .iter()
                .filter_map(|(name, value)| Some((name.as_str().to_ascii_lowercase(), value.to_str().ok()?.to_owned())))
                .collect();
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                let location = headers
                    .iter()
                    .find(|(name, _)| name == "location")
                    .map(|(_, value)| value.as_str())
                    .ok_or_else(|| HttpError(format!("{current} redirected without saying where")))?;
                current = resolve(&current, location)?;
                continue;
            }
            return Ok(Response { status, headers, body: Box::new(response.into_body().into_reader()) });
        }
        Err(HttpError(format!("{url} redirected more than {MAX_REDIRECTS} times")))
    }
}

/// An absolute address, or a path on the same host; nothing else is followed.
fn resolve(from: &str, location: &str) -> Result<String, HttpError> {
    if location.starts_with('/') && !location.starts_with("//") {
        let scheme_end = from.find("://").map_or(0, |at| at + 3);
        let host_end = from[scheme_end..].find(['/', '?', '#']).map_or(from.len(), |at| scheme_end + at);
        return Ok(format!("{}{location}", &from[..host_end]));
    }
    Ok(location.to_owned())
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    const LOOPBACK: Policy = Policy { https_only: false, any_port: true, hosts: &["127.0.0.1"] };
    const BODY: Duration = Duration::from_secs(5);

    fn serve_once(reply: String) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = vec![0u8; 4096];
            let read = stream.read(&mut request).unwrap();
            stream.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&request[..read]).into_owned()
        });
        (format!("http://{address}/releases/latest"), server)
    }

    #[test]
    fn a_body_that_stalls_ends_in_a_read_error_at_the_timeout_instead_of_blocking() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/asset.zip", listener.local_addr().unwrap());
        let (done, released) = std::sync::mpsc::channel::<()>();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = vec![0u8; 4096];
            stream.read(&mut request).unwrap();
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nabc").unwrap();
            let _ = released.recv();
        });

        let started = std::time::Instant::now();
        let response = UreqHttp::with_policy(LOOPBACK).get(&url, "*/*", None, Duration::from_millis(300)).unwrap();
        let error = response.read_limited(100).expect_err("the rest of the body never comes");

        let waited = started.elapsed();
        done.send(()).unwrap();
        server.join().unwrap();
        assert!(error.contains("reading the answer failed"), "{error}");
        assert!(waited < Duration::from_secs(10), "the stall ended at the timeout, not at the server's pleasure: {waited:?}");
    }

    fn redirect_to(location: &str) -> String {
        format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
    }

    #[test]
    fn the_request_names_the_app_and_a_rate_limited_answer_is_an_answer() {
        let (url, server) = serve_once(
            "HTTP/1.1 403 Forbidden\r\nx-ratelimit-reset: 1700000000\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_owned(),
        );

        let response = UreqHttp::with_policy(LOOPBACK).get(&url, "application/vnd.github+json", None, BODY).unwrap();
        let request = server.join().unwrap().to_ascii_lowercase();

        assert_eq!(response.status, 403);
        assert_eq!(response.header("x-ratelimit-reset"), Some("1700000000"));
        assert_eq!(response.read_limited(10).unwrap(), b"{}");
        assert!(request.contains(&format!("user-agent: belisk/{}", env!("CARGO_PKG_VERSION"))), "{request}");
        assert!(request.contains("accept: application/vnd.github+json"), "{request}");
    }

    #[test]
    fn a_range_request_asks_for_the_bytes_from_the_offset() {
        let (url, server) = serve_once("HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc".to_owned());

        let response = UreqHttp::with_policy(LOOPBACK).get(&url, "application/octet-stream", Some(1024), BODY).unwrap();

        assert_eq!(response.status, 206);
        assert!(server.join().unwrap().to_ascii_lowercase().contains("range: bytes=1024-"));
    }

    #[test]
    fn nothing_listening_is_a_transport_error_not_an_answer() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        drop(listener);

        assert!(UreqHttp::with_policy(LOOPBACK).get(&url, "*/*", None, BODY).is_err());
    }

    #[test]
    fn an_answer_longer_than_the_limit_is_refused() {
        let response = Response { status: 200, headers: vec![], body: Box::new(std::io::Cursor::new(vec![0u8; 11])) };
        assert!(response.read_limited(10).is_err());
    }

    #[test]
    fn a_redirect_to_an_allowed_address_is_followed_and_its_headers_kept() {
        let (target, target_server) = serve_once("HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc".to_owned());
        let (url, first_server) = serve_once(redirect_to(&target));

        let response = UreqHttp::with_policy(LOOPBACK).get(&url, "application/octet-stream", Some(7), BODY).unwrap();

        assert_eq!(response.status, 206);
        first_server.join().unwrap();
        assert!(target_server.join().unwrap().to_ascii_lowercase().contains("range: bytes=7-"), "the Range survives the hop");
    }

    #[test]
    fn a_redirect_to_another_host_or_to_http_is_not_followed() {
        for location in ["http://evil.test/payload", "https://127.0.0.1.evil.test/x", "ftp://127.0.0.1/x"] {
            let (url, server) = serve_once(redirect_to(location));

            let error = UreqHttp::with_policy(LOOPBACK).get(&url, "*/*", None, BODY).err().expect("the address is refused");

            server.join().unwrap();
            assert!(error.0.contains("not an address updates may use"), "{location}: {error}");
        }
    }

    #[test]
    fn a_redirect_without_a_location_is_an_error() {
        let (url, server) = serve_once("HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned());

        assert!(UreqHttp::with_policy(LOOPBACK).get(&url, "*/*", None, BODY).is_err());
        server.join().unwrap();
    }

    #[test]
    fn a_relative_redirect_stays_on_the_same_host() {
        assert_eq!(resolve("https://github.com/o/r/releases/download/v1/a.zip", "/o/r/x?y=1").unwrap(), "https://github.com/o/r/x?y=1");
        assert_eq!(resolve("https://github.com", "/x").unwrap(), "https://github.com/x");
        assert_eq!(resolve("https://github.com/a", "https://objects.githubusercontent.com/b").unwrap(), "https://objects.githubusercontent.com/b");
        assert_eq!(resolve("https://github.com/a", "//evil.test/b").unwrap(), "//evil.test/b", "a protocol-relative address is left for the policy to refuse");
        assert!(Policy::GITHUB.check("//evil.test/b").is_err());
    }

    #[test]
    fn github_and_its_download_hosts_over_https_are_the_only_addresses() {
        for allowed in [
            "https://api.github.com/repos/o/r/releases/latest",
            "https://github.com/o/r/releases/download/v1.2.3/belisk-1.2.3-windows-x64-update.zip",
            "https://objects.githubusercontent.com/github-production-release-asset/1?X-Amz=1",
            "https://release-assets.githubusercontent.com/github-production-release-asset/2",
            "HTTPS://GitHub.com/o/r",
        ] {
            assert_eq!(Policy::GITHUB.check(allowed), Ok(()), "{allowed}");
        }
        for refused in [
            "http://api.github.com/repos/o/r/releases/latest",
            "https://evil.test/a.zip",
            "https://github.com.evil.test/a.zip",
            "https://evilgithub.com/a.zip",
            "https://raw.githubusercontent.com/o/r/main/x",
            "https://api.github.com@evil.test/a.zip",
            "https://github.com:8443/a.zip",
            "https://user@github.com/a.zip",
            "https://github.com\\@evil.test/a.zip",
            "https://127.0.0.1/a.zip",
            "https://localhost/a.zip",
            "https://169.254.169.254/latest/meta-data",
            "https:///a.zip",
            "file:///C:/Windows/System32/sethc.exe",
            "ftp://github.com/a.zip",
            "github.com/a.zip",
            "",
        ] {
            assert!(Policy::GITHUB.check(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn the_real_client_refuses_an_address_before_it_sends_anything() {
        let error = UreqHttp::new().get("http://api.github.com/repos/o/r/releases/latest", "*/*", None, BODY).err().expect("the address is refused");

        assert!(error.0.contains("not https"), "{error}");
    }

    fn accept_one_connection_and_hang_up() -> (std::net::SocketAddr, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || drop(listener.accept().unwrap()));
        (address, server)
    }

    #[test]
    fn an_https_request_runs_the_tls_handshake_and_a_failed_one_is_an_error() {
        let (address, server) = accept_one_connection_and_hang_up();

        let error = UreqHttp::with_policy(LOOPBACK).get(&format!("https://{address}/"), "*/*", None, BODY).err().expect("a failed handshake is an error");

        server.join().unwrap();
        assert!(!error.0.is_empty());
    }

    #[test]
    fn a_redirect_to_https_runs_the_tls_handshake_and_a_failed_one_is_an_error() {
        let (address, tls_server) = accept_one_connection_and_hang_up();
        let (url, first_server) = serve_once(redirect_to(&format!("https://{address}/asset.zip")));

        let error = UreqHttp::with_policy(LOOPBACK).get(&url, "*/*", None, BODY).err().expect("a failed handshake is an error");

        first_server.join().unwrap();
        tls_server.join().unwrap();
        assert!(!error.0.contains("not an address updates may use"), "{error}");
    }

    #[test]
    #[ignore = "contacts api.github.com"]
    fn github_answers_the_latest_release_over_schannel() {
        let url = "https://api.github.com/repos/limonanthony/belisk-aion2-companion/releases/latest";

        let response = UreqHttp::new().get(url, "application/vnd.github+json", None, BODY).expect("TLS to api.github.com works");
        let status = response.status;
        let body = String::from_utf8(response.read_limited(1024 * 1024).expect("the body is read")).expect("the body is UTF-8");
        let release: serde_json::Value = serde_json::from_str(&body).expect("the answer is JSON");

        println!("status {status}, tag {}", release["tag_name"]);
        for asset in release["assets"].as_array().expect("assets") {
            println!("asset {} ({} bytes)", asset["name"], asset["size"]);
        }
        assert_eq!(status, 200);
        assert_eq!(release["assets"].as_array().map(Vec::len), Some(4));
    }
}
