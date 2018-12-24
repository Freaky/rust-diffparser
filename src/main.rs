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
    NoNewlineAtEof,
    Junk,
}

fn bytes_to_u32(bytes: &[u8]) -> Option<u32> {
    str::from_utf8(bytes)
        .ok()
        .map(|s| s.trim_matches(|b| !char::is_numeric(b)))
        .and_then(|s| s.parse().ok())
}

#[cfg(unix)]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(bytes))
}

#[cfg(windows)]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).to_string())
}

fn parse_old_file(line: &[u8]) -> DiffLine<'_> {
    if line.len() > 4 {
        if let b"--- " = &line[0..4] {
            let eof = line
                .iter()
                .position(|&b| b == b'\t' || b == b'\r' || b == b'\n')
                .unwrap_or_else(|| line.len());

            return DiffLine::OldFile(FileInfo {
                filename: bytes_to_pathbuf(&line[4..eof]),
            });
        }
    }

    DiffLine::Junk
}

fn parse_new_file(line: &[u8]) -> DiffLine<'_> {
    if line.len() > 4 {
        if let b"+++ " = &line[0..4] {
            let eof = line
                .iter()
                .position(|&b| b == b'\t' || b == b'\r' || b == b'\n')
                .unwrap_or_else(|| line.len());

            return DiffLine::NewFile(FileInfo {
                filename: bytes_to_pathbuf(&line[4..eof]),
            });
        }
    }

    DiffLine::Junk
}

fn parse_hunk(line: &[u8]) -> DiffLine<'_> {
    if line.len() > 11 {
        if let b"@@ -1 +1 @@" = &line[0..11] {
            return DiffLine::Hunk(HunkInfo {
                old_line_no: 1,
                new_line_no: 1,
                old_line_len: 1,
                new_line_len: 1,
            });
        }

        if let b"@@ -" = &line[0..4] {
            // svn also has ## for properties
            // @@ -1,1 +1,1 @@
            // @@ -1 +1 @@
            let mut chunks = line[3..]
                .split(|&b| b == b' ' || b == b',')
                .flat_map(bytes_to_u32);

            return DiffLine::Hunk(HunkInfo {
                old_line_no: chunks.next().unwrap_or_default(),
                old_line_len: chunks.next().unwrap_or_default(),
                new_line_no: chunks.next().unwrap_or_default(),
                new_line_len: chunks.next().unwrap_or_default(),
            });
        }
    }

    DiffLine::Junk
}

fn parse_delta(line: &[u8]) -> DiffLine<'_> {
    match line[0] {
        b'+' => DiffLine::Inserted(&line[1..]),
        b'-' => DiffLine::Deleted(&line[1..]),
        b'!' => DiffLine::Modified(&line[1..]),
        b' ' => DiffLine::Context(&line[1..]),
        b'\\' => DiffLine::NoNewlineAtEof,
        _ => DiffLine::Junk,
    }
}

#[derive(Debug)]
enum State {
    Junk,
    OldFile,
    NewFile,
    Hunk(i32, i32),
}

use std::io;
use std::io::BufRead;

struct DiffParser<R> {
    inner: R,
    state: State,
    line: Vec<u8>,
}

impl<R: BufRead> DiffParser<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            state: State::Junk,
            line: vec![],
        }
    }

    fn next_line(&mut self) -> Option<io::Result<DiffLine>> {
        self.line.clear();

        let parsed = self.inner.read_until(b'\n', &mut self.line);

        match parsed {
            Ok(0) => return None,
            Ok(_) => (),
            Err(err) => return Some(Err(err)),
        };

        match self.state {
            State::Junk => {
                let line = parse_old_file(&self.line[..]);
                if let DiffLine::OldFile(_) = line {
                    self.state = State::OldFile;
                }

                Some(Ok(line))
            }
            State::OldFile => {
                let line = parse_new_file(&self.line[..]);

                if let DiffLine::NewFile(_) = line {
                    self.state = State::NewFile;
                } else {
                    self.state = State::Junk;
                }

                Some(Ok(line))
            }
            State::NewFile => {
                let line = parse_hunk(&self.line[..]);

                if let DiffLine::Hunk(ref info) = line {
                    self.state = State::Hunk(info.old_line_len as i32, info.new_line_len as i32);
                } else {
                    self.state = State::Junk;
                }

                Some(Ok(line))
            }
            State::Hunk(ref mut old, ref mut new) => {
                let line = parse_delta(&self.line[..]);
                match line {
                    DiffLine::Context(_) | DiffLine::Modified(_) => {
                        *old -= 1;
                        *new -= 1;
                    }
                    DiffLine::Inserted(_) => {
                        *new -= 1;
                    }
                    DiffLine::Deleted(_) => {
                        *old -= 1;
                    }
                    DiffLine::NoNewlineAtEof => (),
                    _ => {
                        println!(
                            "JUNK IN THE HUNK! state={:?}, line={:?}, raw={}",
                            self.state,
                            line,
                            String::from_utf8_lossy(&self.line[..])
                        );
                        self.state = State::Junk;
                        return Some(Ok(DiffLine::Junk));
                    }
                };

                if (*old < 0 || *new < 0) || (*old == 0 && *new == 0) {
                    self.state = State::NewFile;
                }

                Some(Ok(line))
            }
        }
    }
}

use std::io::BufReader;
fn diffstat<R: std::io::BufRead>(diff: R) {
    let mut files = 0;
    let mut hunks = 0;
    let mut insert = 0;
    let mut delete = 0;
    let mut modify = 0;

    let mut parser = DiffParser::new(diff);

    while let Some(line) = parser.next_line() {
        let line = line.expect("read error");
        match line {
            DiffLine::Inserted(_) => insert += 1,
            DiffLine::Deleted(_) => delete += 1,
            DiffLine::Modified(_) => modify += 1,
            DiffLine::Hunk(_) => hunks += 1,
            DiffLine::NewFile(_) => files += 1,
            DiffLine::Junk => {
                // eprintln!("JUNK: line={}", String::from_utf8_lossy(&parser.line[..]));
            }
            _ => (),
        }
    }

    println!(
        "{} file(s) changed, {} hunks, {} insertions(+), {} deletions(-), {} modifications(!)",
        files, hunks, insert, delete, modify
    );
}

use std::env;
use std::fs::File;

fn main() {
    let filename = env::args_os().nth(1).expect("Need a path");

    let diff = File::open(filename).expect("open error");
    diffstat(BufReader::new(diff));
}
