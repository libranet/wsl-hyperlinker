//! Tests for the hyperlink rewriter.

use wsl_hyperlinker::Rewriter;

/// ST terminator: `ESC \`.
const ST: &str = "\x1b\\";
/// BEL terminator.
const BEL: &str = "\x07";

/// A rewriter for distro `Distro` on host `mypc`.
fn rw() -> Rewriter {
    Rewriter::new("Distro", "mypc")
}

/// A hyperlink to `uri` with text `name`, terminated by `st`.
fn link(uri: &str, st: &str) -> String {
    format!("\x1b]8;;{uri}{st}name\x1b]8;;{st}")
}

/// Rewrites `input` in one chunk.
fn run(input: &str) -> String {
    let mut r = rw();
    let mut out = r.feed(input.as_bytes());
    out.extend(r.finish());
    String::from_utf8(out).unwrap()
}

#[test]
fn linux_paths() {
    assert_eq!(run(&link("file:///home/me/a", ST)), link("file://wsl.localhost/Distro/home/me/a", ST));
    assert_eq!(run(&link("file://MYPC/home/me/a", BEL)), link("file://wsl.localhost/Distro/home/me/a", BEL));
    assert_eq!(run(&link("file://localhost/etc", BEL)), link("file://wsl.localhost/Distro/etc", BEL));
}

#[test]
fn windows_drives() {
    assert_eq!(run(&link("file:///mnt/c/Windows", BEL)), link("file:///C:/Windows", BEL));
    assert_eq!(run(&link("file:///mnt/d", BEL)), link("file:///D:/", BEL));
    assert_eq!(run(&link("file:///mnt/wsl/x", BEL)), link("file://wsl.localhost/Distro/mnt/wsl/x", BEL));
}

#[test]
fn unchanged() {
    for uri in ["file://otherhost/x", "https://example.com/", "file:relative"] {
        assert_eq!(run(&link(uri, ST)), link(uri, ST));
    }
    assert_eq!(run("plain \x1b[31mred\x1b[0m text\n"), "plain \x1b[31mred\x1b[0m text\n");
}

#[test]
fn distro_name_percent_encoded() {
    let mut r = Rewriter::new("My Distro#1%", "mypc");
    let mut out = r.feed(link("file:///tmp", BEL).as_bytes());
    out.extend(r.finish());
    assert_eq!(String::from_utf8(out).unwrap(), link("file://wsl.localhost/My%20Distro%231%25/tmp", BEL));
}

#[test]
fn params_kept() {
    let s = "\x1b]8;id=1;file:///tmp\x07x\x1b]8;;\x07";
    assert_eq!(run(s), "\x1b]8;id=1;file://wsl.localhost/Distro/tmp\x07x\x1b]8;;\x07");
}

#[test]
fn unterminated_sequence_released_by_finish() {
    assert_eq!(run("text \x1b]8;;file:///tmp"), "text \x1b]8;;file:///tmp");
    assert_eq!(run("text \x1b]8"), "text \x1b]8");
}

#[test]
fn split_across_chunks() {
    let input = format!("abc {} def", link("file:///home/me/a", ST));
    let want = format!("abc {} def", link("file://wsl.localhost/Distro/home/me/a", ST));
    for size in 1..input.len() {
        let mut r = rw();
        let mut out = Vec::new();
        for chunk in input.as_bytes().chunks(size) {
            out.extend(r.feed(chunk));
        }
        out.extend(r.finish());
        assert_eq!(String::from_utf8(out).unwrap(), want, "chunk size {size}");
    }
}
