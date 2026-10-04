//! Streaming rewriter for OSC 8 hyperlinks.
//!
//! A hyperlink is written as `ESC ] 8 ; params ; uri ST`, where ST (the string terminator) is
//! either BEL (`\x07`) or `ESC \`. Spec: <https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda>

use std::mem;

use crate::sys;

/// Start of an OSC 8 hyperlink sequence: `ESC ] 8 ;`.
const OSC8: &[u8] = b"\x1b]8;";

/// An unterminated link sequence longer than this is passed through unchanged.
const MAX_PENDING: usize = 16 * 1024;

/// Rewrites `file://` hyperlinks in a byte stream so Windows can open them.
///
/// Output may be split at any byte, so an incomplete link sequence at the end of a chunk is held
/// back until the next [`feed`](Self::feed) call, or released by [`finish`](Self::finish).
///
/// # Example
///
/// ```
/// use wsl_hyperlinker::Rewriter;
///
/// let mut rw = Rewriter::new("Ubuntu", "mypc");
/// let mut out = rw.feed(b"\x1b]8;;file:///mnt/c/Users\x07Users\x1b]8;;\x07");
/// out.extend(rw.finish());
/// assert_eq!(out, b"\x1b]8;;file:///C:/Users\x07Users\x1b]8;;\x07");
/// ```
#[derive(Debug)]
pub struct Rewriter {
    /// WSL distribution name, percent-encoded, used in `file://wsl.localhost/<distro>/...`.
    distro: String,
    /// This machine's host name, lowercase; links to it count as local.
    hostname: String,
    /// Bytes held back from the previous chunk (an incomplete link sequence).
    pending: Vec<u8>,
}

impl Rewriter {
    /// Creates a rewriter for `distro` that treats links to `hostname` (any case) as local.
    #[must_use]
    pub fn new(distro: &str, hostname: &str) -> Self {
        Rewriter { distro: percent_encode(distro), hostname: hostname.to_ascii_lowercase(), pending: Vec::new() }
    }

    /// Creates a rewriter for `distro` on this machine, using its current host name.
    #[must_use]
    pub fn for_local_host(distro: &str) -> Self {
        Self::new(distro, &sys::hostname())
    }

    /// Processes a chunk and returns the bytes that are ready to be written.
    ///
    /// Bytes that may belong to an incomplete link sequence are kept for the next call.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<u8> {
        self.pending.extend_from_slice(chunk);
        let buf = mem::take(&mut self.pending);
        let mut out = Vec::with_capacity(buf.len() + 64);
        let mut i = 0;

        while let Some(off) = find(&buf[i..], OSC8) {
            let start = i + off;
            out.extend_from_slice(&buf[i..start]);
            let body = start + OSC8.len();
            match find_terminator(&buf[body..]) {
                Some(end) => {
                    let seq = &buf[body..body + end]; // "params;uri"
                    self.rewrite_sequence(seq, &mut out);
                    i = body + end; // terminator is copied with the following text
                }
                None if buf.len() - start < MAX_PENDING => {
                    self.pending = buf[start..].to_vec();
                    return out;
                }
                None => {
                    out.extend_from_slice(&buf[start..body]);
                    i = body;
                }
            }
        }

        // Hold back a trailing partial "\x1b]8;" that may complete in the next chunk.
        let rest = &buf[i..];
        let keep = partial_prefix_len(rest, OSC8);
        out.extend_from_slice(&rest[..rest.len() - keep]);
        self.pending = rest[rest.len() - keep..].to_vec();
        out
    }

    /// Returns any bytes still held back. Call this once the input has ended.
    pub fn finish(&mut self) -> Vec<u8> {
        mem::take(&mut self.pending)
    }

    /// Writes `OSC8` + `seq` (the `params;uri` part of one sequence) to `out`, with the URI rewritten.
    fn rewrite_sequence(&self, seq: &[u8], out: &mut Vec<u8>) {
        out.extend_from_slice(OSC8);
        let Some(semi) = seq.iter().position(|&b| b == b';') else {
            out.extend_from_slice(seq);
            return;
        };
        let (params, uri) = (&seq[..=semi], &seq[semi + 1..]);
        out.extend_from_slice(params);
        match str::from_utf8(uri).ok().and_then(|u| self.rewrite_uri(u)) {
            Some(new) => out.extend_from_slice(new.as_bytes()),
            None => out.extend_from_slice(uri),
        }
    }

    /// Returns the rewritten URI, or `None` to keep it unchanged.
    fn rewrite_uri(&self, uri: &str) -> Option<String> {
        let rest = uri.strip_prefix("file://")?;
        let slash = rest.find('/')?;
        let (host, path) = (&rest[..slash], &rest[slash..]);
        let host = host.to_ascii_lowercase();
        if !(host.is_empty() || host == "localhost" || host == self.hostname) {
            return None; // link to another machine
        }
        if let Some(uri) = windows_drive_uri(path) {
            return Some(uri);
        }
        Some(format!("file://wsl.localhost/{}{}", self.distro, path))
    }
}

/// Maps `/mnt/<drive>[/...]` to `file:///<DRIVE>:/...`; `None` for any other path.
fn windows_drive_uri(path: &str) -> Option<String> {
    let after = path.strip_prefix("/mnt/")?;
    let mut chars = after.chars();
    let drive = chars.next().filter(char::is_ascii_alphabetic)?;
    let tail = match chars.as_str() {
        "" => "/",
        tail if tail.starts_with('/') => tail,
        _ => return None, // e.g. /mnt/wsl: not a drive
    };
    Some(format!("file:///{}:{}", drive.to_ascii_uppercase(), tail))
}

/// Percent-encodes `s` for use as one URI path segment: every byte except the RFC 3986
/// unreserved characters (`A-Z a-z 0-9 - . _ ~`) becomes `%XX`.
fn percent_encode(s: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}"); // writing to a String cannot fail
        }
    }
    out
}

/// Position of the first occurrence of `needle` in `hay`.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Position of the OSC terminator: BEL (`0x07`) or ST (`ESC \`).
fn find_terminator(buf: &[u8]) -> Option<usize> {
    buf.iter().enumerate().find_map(|(i, &b)| match b {
        0x07 => Some(i),
        0x1b if buf.get(i + 1) == Some(&b'\\') => Some(i),
        _ => None,
    })
}

/// Length of the longest suffix of `buf` that is a proper prefix of `needle`.
fn partial_prefix_len(buf: &[u8], needle: &[u8]) -> usize {
    (1..needle.len().min(buf.len() + 1)).rev().find(|&n| buf.ends_with(&needle[..n])).unwrap_or(0)
}
