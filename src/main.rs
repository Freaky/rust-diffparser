
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::str;

#[derive(Debug)]
pub struct FileInfo {
    pub filename: PathBuf,
}

#[derive(Debug)]
pub struct HunkInfo {
    pub old_line_no: u32,
    pub old_line_len: u32,
    pub new_line_no: u32,
    pub new_line_len: u32,
}

#[derive(Debug)]
pub enum DiffLine<'a> {
    OldFile(FileInfo),
    NewFile(FileInfo),
    Hunk(HunkInfo),
    Context(&'a [u8]),
    Inserted(&'a [u8]),
    Deleted(&'a [u8]),
    Modified(&'a [u8]),
    Skipped,
}

fn bytes_to_u32(bytes: &[u8]) -> Option<u32> {
    str::from_utf8(bytes)
        .ok()
        .map(|s| s.trim_matches(|b| !char::is_numeric(b)))
        .and_then(|s| s.parse().ok())
}

pub fn parse_diff_line(line: &[u8]) -> DiffLine<'_> {
    if line.is_empty() {
        return DiffLine::Skipped;
    }

    if line.len() > 4 {
        match &line[0..4] {
            b"--- " => {
                let eof = line.iter().rposition(|&b| b == b'\t').unwrap_or_else(|| line.len());
                return DiffLine::OldFile(FileInfo {
                    filename: PathBuf::from(OsStr::from_bytes(&line[4..eof])),
                });
            }
            b"+++ " => {
                let eof = line.iter().rposition(|&b| b == b'\t').unwrap_or_else(|| line.len());
                return DiffLine::NewFile(FileInfo {
                    filename: PathBuf::from(OsStr::from_bytes(&line[4..eof])),
                });
            }
            b"@@ -" if line.len() > 15 => { // svn also has ## for properties
                // @@ -1,1 +1,1 @@
                let mut chunks = line[3..].split(|&b| b == b' ' || b == b',');
                let old_line_no = chunks.next().and_then(bytes_to_u32).unwrap_or_default();
                let old_line_len = chunks.next().and_then(bytes_to_u32).unwrap_or_default();
                let new_line_no = chunks.next().and_then(bytes_to_u32).unwrap_or_default();
                let new_line_len = chunks.next().and_then(bytes_to_u32).unwrap_or_default();

                return DiffLine::Hunk(HunkInfo {
                    old_line_no,
                    new_line_no,
                    old_line_len,
                    new_line_len,
                });
            }
            _ => (),
        }
    }

    match line[0] {
        b'+' => DiffLine::Inserted(&line[1..]),
        b'-' => DiffLine::Deleted(&line[1..]),
        b'!' => DiffLine::Modified(&line[1..]),
        b' ' => DiffLine::Context(&line[1..]),
        _ => DiffLine::Skipped,
    }
}

use std::io::BufReader;
fn diffstat<R: std::io::BufRead>(diff: R) {
    let mut files = 0;
    let mut hunks = 0;
    let mut insert = 0;
    let mut delete = 0;
    let mut modify = 0;

    for line in diff.split(b'\n') {
        let line = line.expect("read error");
        let parsed = parse_diff_line(&line[..]);
        match parsed {
            DiffLine::Inserted(_) => insert += 1,
            DiffLine::Deleted(_) => delete += 1,
            DiffLine::Modified(_) => modify += 1,
            DiffLine::Hunk(_) => hunks += 1,
            DiffLine::NewFile(_) => files += 1,
            _ => ()
        }
    }

    println!("{} file(s) changed, {} hunks, {} insertions(+), {} deletions(-), {} modifications(!)", files, hunks, insert, delete, modify);
}


use std::env;
use std::fs::File;

fn main() {
    let filename = env::args_os().nth(1).expect("Need a path");

    let diff = File::open(filename).expect("open error");
    diffstat(BufReader::new(diff));

    // for line in diff.split(|&b| b == b'\n') {
        // println!("{:?}", parse_diff_line(line));
    // }
}
