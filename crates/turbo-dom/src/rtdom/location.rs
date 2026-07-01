//! Pure-Rust port of the Location + History surface from
//! `src/runtime/stubs.mjs` (`makeLocation` / `makeHistory`).
//!
//! Standalone: no Tree dependency, no wasm/napi, no external crates. The JS
//! relies on the host `URL`; here we hand-parse the href (no `url` crate in
//! deps). Field semantics mirror the WHATWG `URL` fields the JS exposes:
//!   * `href`     — the parsed/normalized href
//!   * `protocol` — scheme + ":" (e.g. "https:")
//!   * `host`     — hostname[:port]
//!   * `hostname` — host without port
//!   * `port`     — port digits ("" if none)
//!   * `pathname` — path (defaults "/")
//!   * `search`   — query incl. leading "?" ("" if none)
//!   * `hash`     — fragment incl. leading "#" ("" if none)
//!   * `origin`   — protocol + "//" + host

/// Parsed location surface (mirrors `makeLocation`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Location {
    pub href: String,
    pub protocol: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
    pub origin: String,
}

impl Location {
    /// Hand-parse an href into its component fields.
    ///
    /// Grammar handled: `scheme:` then optional `//authority`, then
    /// `path`, then optional `?query`, then optional `#fragment`.
    #[must_use]
    pub fn parse(href: &str) -> Location {
        // Split off the fragment first (everything after the first '#').
        let (before_hash, hash) = match href.find('#') {
            Some(i) => (&href[..i], href[i..].to_string()),
            None => (href, String::new()),
        };

        // Split off the query (everything after the first '?').
        let (before_query, search) = match before_hash.find('?') {
            Some(i) => (&before_hash[..i], before_hash[i..].to_string()),
            None => (before_hash, String::new()),
        };

        // Scheme: leading run up to the first ':'.
        let (protocol, after_scheme) = match before_query.find(':') {
            Some(i) => {
                let scheme = &before_query[..=i]; // include the ':'
                (scheme.to_string(), &before_query[i + 1..])
            }
            None => (String::new(), before_query),
        };

        // Optional `//authority`.
        let (authority, pathname) = if let Some(rest) = after_scheme.strip_prefix("//") {
            // Authority runs until the next '/' (or end-of-string).
            match rest.find('/') {
                Some(i) => (&rest[..i], &rest[i..]),
                None => (rest, ""),
            }
        } else {
            // No authority: everything left is the path.
            ("", after_scheme)
        };

        // Split authority into hostname:port (port = digits after the last ':').
        let (hostname, port) = match authority.rfind(':') {
            Some(i) => (authority[..i].to_string(), authority[i + 1..].to_string()),
            None => (authority.to_string(), String::new()),
        };

        // host = hostname[:port]
        let host = if port.is_empty() {
            hostname.clone()
        } else {
            format!("{hostname}:{port}")
        };

        // pathname defaults to "/".
        let pathname = if pathname.is_empty() {
            "/".to_string()
        } else {
            pathname.to_string()
        };

        // origin = protocol + "//" + host (mirrors URL.origin for hierarchical URLs).
        let origin = format!("{protocol}//{host}");

        Location {
            href: href.to_string(),
            protocol,
            host,
            hostname,
            port,
            pathname,
            search,
            hash,
            origin,
        }
    }
}

/// One entry on the history stack.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    state: String,
    url: String,
}

/// Browser-like history stack (mirrors `makeHistory`).
#[derive(Debug, Clone)]
pub struct History {
    stack: Vec<Entry>,
    idx: usize,
    location: Location,
}

impl History {
    /// Seed the stack with the initial location (state = "").
    #[must_use]
    pub fn new(initial: Location) -> History {
        let url = initial.href.clone();
        History {
            stack: vec![Entry {
                state: String::new(),
                url,
            }],
            idx: 0,
            location: initial,
        }
    }

    /// Push a new entry, truncating any forward entries (browser semantics).
    pub fn push_state(&mut self, state: String, url: Option<&str>) {
        let url = url.map_or_else(|| self.current_url(), str::to_string);
        self.stack.truncate(self.idx + 1);
        self.stack.push(Entry { state, url });
        self.idx += 1;
    }

    /// Replace the current entry in place (url defaults to the current url).
    pub fn replace_state(&mut self, state: String, url: Option<&str>) {
        let url = url.map_or_else(|| self.stack[self.idx].url.clone(), str::to_string);
        self.stack[self.idx] = Entry { state, url };
    }

    /// Move back one entry (clamped at 0).
    pub fn back(&mut self) {
        if self.idx > 0 {
            self.idx -= 1;
        }
    }

    /// Move forward one entry (clamped at the top).
    pub fn forward(&mut self) {
        if self.idx < self.stack.len() - 1 {
            self.idx += 1;
        }
    }

    /// Move by `delta` entries, clamped to `[0, len-1]`.
    pub fn go(&mut self, delta: i32) {
        let target = self.idx as i64 + i64::from(delta);
        let max = self.stack.len() as i64 - 1;
        let clamped = target.max(0).min(max);
        self.idx = clamped as usize;
    }

    /// Number of entries on the stack.
    #[must_use]
    pub fn length(&self) -> usize {
        self.stack.len()
    }

    /// The location this history was seeded with.
    #[must_use]
    pub fn current(&self) -> &Location {
        &self.location
    }

    /// The state of the current entry.
    #[must_use]
    pub fn state(&self) -> &str {
        &self.stack[self.idx].state
    }

    fn current_url(&self) -> String {
        self.stack[self.idx].url.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_url() {
        let l = Location::parse("https://a.com:8080/p/q?x=1&y=2#sec");
        assert_eq!(l.href, "https://a.com:8080/p/q?x=1&y=2#sec");
        assert_eq!(l.protocol, "https:");
        assert_eq!(l.host, "a.com:8080");
        assert_eq!(l.hostname, "a.com");
        assert_eq!(l.port, "8080");
        assert_eq!(l.pathname, "/p/q");
        assert_eq!(l.search, "?x=1&y=2");
        assert_eq!(l.hash, "#sec");
        assert_eq!(l.origin, "https://a.com:8080");
    }

    #[test]
    fn parse_localhost_no_port() {
        let l = Location::parse("http://localhost/");
        assert_eq!(l.protocol, "http:");
        assert_eq!(l.host, "localhost");
        assert_eq!(l.hostname, "localhost");
        assert_eq!(l.port, "");
        assert_eq!(l.pathname, "/");
        assert_eq!(l.search, "");
        assert_eq!(l.hash, "");
        assert_eq!(l.origin, "http://localhost");
    }

    #[test]
    fn parse_authority_no_path() {
        // No trailing slash after authority -> pathname defaults to "/".
        let l = Location::parse("https://example.com");
        assert_eq!(l.hostname, "example.com");
        assert_eq!(l.port, "");
        assert_eq!(l.pathname, "/");
        assert_eq!(l.host, "example.com");
        assert_eq!(l.origin, "https://example.com");
    }

    #[test]
    fn parse_path_only_relative() {
        // No scheme, no authority: everything is the path.
        let l = Location::parse("/foo/bar?q=1#frag");
        assert_eq!(l.protocol, "");
        assert_eq!(l.host, "");
        assert_eq!(l.hostname, "");
        assert_eq!(l.port, "");
        assert_eq!(l.pathname, "/foo/bar");
        assert_eq!(l.search, "?q=1");
        assert_eq!(l.hash, "#frag");
        assert_eq!(l.origin, "//");
    }

    #[test]
    fn parse_default_location() {
        // Default impl gives all-empty fields.
        let l = Location::default();
        assert_eq!(l.href, "");
        assert_eq!(l.pathname, "");
    }

    #[test]
    fn history_push_replace_back_forward_go() {
        let l = Location::parse("http://localhost/");
        let mut h = History::new(l.clone());
        assert_eq!(h.length(), 1);
        assert_eq!(h.state(), "");
        assert_eq!(h.current(), &l);

        // push two entries
        h.push_state("s1".to_string(), Some("/a"));
        h.push_state("s2".to_string(), None); // url defaults to current url ("/a")
        assert_eq!(h.length(), 3);
        assert_eq!(h.state(), "s2");

        // back twice (second back clamps progress, not below 0)
        h.back();
        assert_eq!(h.state(), "s1");
        h.back();
        assert_eq!(h.state(), "");
        h.back(); // clamp at 0
        assert_eq!(h.state(), "");

        // forward
        h.forward();
        assert_eq!(h.state(), "s1");

        // replace current entry (no url -> keeps current url)
        h.replace_state("s1b".to_string(), None);
        assert_eq!(h.state(), "s1b");
        h.replace_state("s1c".to_string(), Some("/changed"));
        assert_eq!(h.state(), "s1c");

        // push from idx=1 truncates the forward entry (was "s2"), then re-adds
        h.push_state("s3".to_string(), Some("/c"));
        assert_eq!(h.length(), 3);
        assert_eq!(h.state(), "s3");

        // forward at the top is a no-op (clamp)
        h.forward();
        assert_eq!(h.state(), "s3");

        // go: negative, positive, and over-shoot clamps
        h.go(-2);
        assert_eq!(h.state(), "");
        h.go(100); // clamp to top
        assert_eq!(h.state(), "s3");
        h.go(-100); // clamp to 0
        assert_eq!(h.state(), "");
        h.go(0); // no movement
        assert_eq!(h.state(), "");
    }
}
