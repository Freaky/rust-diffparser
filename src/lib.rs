
#[derive(Debug)]
pub struct FileInfo<'a> {
    pub filename: &'a [u8],
    pub metadata: Option<&'a [u8]>
}

#[derive(Debug, Default)]
pub struct HunkInfo {
    pub old_line_no: u32,
    pub old_line_len: u32,
    pub new_line_no: u32,
    pub new_line_len: u32,
}

#[derive(Debug)]
pub enum DiffLine<'a> {
    OldFile(FileInfo<'a>),
    NewFile(FileInfo<'a>),
    Binaries(&'a [u8], &'a [u8]),
    Hunk(HunkInfo),
    Context(&'a [u8]),
    Inserted(&'a [u8]),
    Deleted(&'a [u8]),
    Modified(&'a [u8]),
    NoNewlineAtEof,
    Junk,
}

fn bytes_to_u32(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() {
        return None;
    }

    bytes.iter().try_fold(0u32, |acc, byte| {
        acc.checked_mul(10).and_then(|acc| {
            (*byte as char)
                .to_digit(10)
                .and_then(|digit| acc.checked_add(digit))
        })
    })
}

#[test]
fn test_bytes_to_u32() {
    assert_eq!(bytes_to_u32(b"0"), Some(0));
    assert_eq!(bytes_to_u32(b"10"), Some(10));
    assert_eq!(bytes_to_u32(b"42"), Some(42));
    assert_eq!(bytes_to_u32(b"4294967295"), Some(4294967295));
    assert_eq!(bytes_to_u32(b"4294967296"), None);
    assert_eq!(bytes_to_u32(b"12345six"), None);
    assert_eq!(bytes_to_u32(b"nope"), None);
    assert_eq!(bytes_to_u32(b""), None);
}

fn parse_fileinfo(line: &[u8]) -> FileInfo<'_> {
   let eof = line
        .iter()
        .position(|&b| b == b'\t' || b == b'\r' || b == b'\n')
        .unwrap_or_else(|| line.len());

    FileInfo {
        filename: &line[4..eof],
        metadata: if eof != line.len() { Some(&line[eof..line.len()]) } else { None }
    }
}

fn parse_old_file(line: &[u8]) -> DiffLine<'_> {
    if line.starts_with(b"Binary files ") && line.ends_with(b"differ\n") {
        // Binary files sigh and blegh differ
        let x = &line[b"Binary files ".len()..line.len() - b"differ\n".len()];
        if let Some(pos) = x.windows(b" and ".len()).position(|win| win == b" and ") {
            return DiffLine::Binaries(
                &x[0..pos],
                &x[pos..],
            );
        }
    }

    if line.starts_with(b"--- ") {
        return DiffLine::OldFile(parse_fileinfo(line));
    }

    DiffLine::Junk
}

fn parse_new_file(line: &[u8]) -> DiffLine<'_> {
    if line.starts_with(b"+++ ") {
        return DiffLine::NewFile(parse_fileinfo(line));
    }

    DiffLine::Junk
}

fn parse_hunk(line: &[u8]) -> DiffLine<'_> {
    if line.len() > b"@@ -1 +1 @@".len() && line.starts_with(b"@@ -") {
        // svn also has ## for properties
        // @@ -1,1 +1,1 @@
        // @@ -1 +1 @@

        let mut hunk = HunkInfo::default();

        let mut chunks = line[4..]
            .split(|&b| b == b' ')
            .map(|chunk| chunk.split(|&b| b == b','));

        if let (Some(mut old), Some(mut new)) = (chunks.next(), chunks.next()) {
            if let Some(oln) = old.next().and_then(bytes_to_u32) {
                hunk.old_line_no = oln;
                hunk.old_line_len = old.next().and_then(bytes_to_u32).unwrap_or(1);
            } else {
                return DiffLine::Junk;
            }

            if let Some(nln) = new.next().and_then(|b| bytes_to_u32(&b[1..])) {
                hunk.new_line_no = nln;
                hunk.new_line_len = new.next().and_then(bytes_to_u32).unwrap_or(1);
            } else {
                return DiffLine::Junk;
            }

            return DiffLine::Hunk(hunk);
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

pub struct DiffParser<R> {
    inner: R,
    state: State,
    pub line: Vec<u8>,
}

impl<R: BufRead> DiffParser<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            state: State::Junk,
            line: vec![],
        }
    }

    pub fn next_line(&mut self) -> Option<io::Result<DiffLine>> {
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
