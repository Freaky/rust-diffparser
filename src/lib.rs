use std::fmt;

#[derive(Debug, PartialEq)]
pub struct FileInfo<'a> {
    pub filename: &'a [u8],
    pub metadata: Option<&'a [u8]>,
}

#[derive(Debug, Default, PartialEq)]
pub struct HunkInfo<'a> {
    pub old_line_no: u32,
    pub old_line_len: u32,
    pub new_line_no: u32,
    pub new_line_len: u32,
    pub context: Option<&'a [u8]>,
}

#[derive(Debug, PartialEq)]
pub enum DiffLine<'a> {
    OldFile(FileInfo<'a>),
    NewFile(FileInfo<'a>),
    Binaries(&'a [u8], &'a [u8]),
    Hunk(HunkInfo<'a>),
    Context(&'a [u8]),
    Inserted(&'a [u8]),
    Deleted(&'a [u8]),
    Modified(&'a [u8]),
    NoNewlineAtEof,
    Junk(&'a [u8]),
}

impl fmt::Display for HunkInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "@@ -{}", self.old_line_no)?;
        if self.old_line_len > 1 {
            write!(f, ",{}", self.old_line_len)?;
        }

        write!(f, " +{}", self.new_line_no)?;
        if self.new_line_len > 1 {
            write!(f, ",{}", self.new_line_len)?;
        }
        write!(f, " @@")?;

        if let Some(ctx) = self.context {
            write!(f, "\t{}", String::from_utf8_lossy(ctx))?;
        }

        Ok(())
    }
}

impl fmt::Display for FileInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.filename))?;
        if let Some(md) = self.metadata {
            write!(f, "\t{}", String::from_utf8_lossy(md))?;
        }
        Ok(())
    }
}

impl fmt::Display for DiffLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DiffLine::OldFile(fi) => write!(f, "--- {}", fi),
            DiffLine::NewFile(fi) => write!(f, "+++ {}", fi),
            DiffLine::Binaries(a, b) => writeln!(
                f,
                "Binary files {} and {} differ",
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b)
            ),
            DiffLine::Hunk(hi) => write!(f, "{}", hi),
            DiffLine::Context(l) => write!(f, " {}", String::from_utf8_lossy(l)),
            DiffLine::Inserted(l) => write!(f, "+{}", String::from_utf8_lossy(l)),
            DiffLine::Deleted(l) => write!(f, "-{}", String::from_utf8_lossy(l)),
            DiffLine::Modified(l) => write!(f, "!{}", String::from_utf8_lossy(l)),
            DiffLine::NoNewlineAtEof => writeln!(f, "\\ No newline at end of file"),
            DiffLine::Junk(l) => write!(f, "{}", String::from_utf8_lossy(l)),
        }
    }
}

fn chomp(slice: &[u8]) -> &[u8] {
    if slice.ends_with(b"\r\n") {
        &slice[..slice.len() - 2]
    } else if slice.ends_with(b"\n") {
        &slice[..slice.len() - 1]
    } else {
        slice
    }
}

#[test]
fn test_chomp() {
    assert_eq!(chomp(b""), b"");
    assert_eq!(chomp(b"foo"), b"foo");
    assert_eq!(chomp(b"foo\r\n"), b"foo");
    assert_eq!(chomp(b"foo\n"), b"foo");
}

fn parse_u32(bytes: &[u8]) -> Option<u32> {
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
fn test_parse_u32() {
    assert_eq!(parse_u32(b"0"), Some(0));
    assert_eq!(parse_u32(b"10"), Some(10));
    assert_eq!(parse_u32(b"42"), Some(42));
    assert_eq!(parse_u32(b"4294967295"), Some(4294967295));
    assert_eq!(parse_u32(b"4294967296"), None);
    assert_eq!(parse_u32(b"12345six"), None);
    assert_eq!(parse_u32(b"nope"), None);
    assert_eq!(parse_u32(b""), None);
}

fn parse_range(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut bits = bytes.split(|&b| b == b',').flat_map(parse_u32);

    Some((bits.next()?, bits.next().unwrap_or(1)))
}

#[test]
fn test_parse_range() {
    assert_eq!(parse_range(b"12,24"), Some((12, 24)));
    assert_eq!(parse_range(b"12"), Some((12, 1)));
    assert_eq!(parse_range(b"42,"), Some((42, 1)));
    assert_eq!(parse_range(b","), None);
    assert_eq!(parse_range(b""), None);
}

fn parse_fileinfo(line: &[u8]) -> FileInfo<'_> {
    let eof = line
        .iter()
        .position(|&b| b == b'\t' || b == b'\r' || b == b'\n')
        .unwrap_or_else(|| line.len());

    FileInfo {
        filename: &line[4..eof],
        metadata: if eof != line.len() - 1 {
            Some(&line[eof + 1..line.len() - 1])
        } else {
            None
        },
    }
}

fn parse_old_file(line: &[u8]) -> DiffLine<'_> {
    if line.starts_with(b"Binary files ") && line.ends_with(b"differ\n") {
        // Binary files sigh and blegh differ
        let x = &line[b"Binary files ".len()..line.len() - b" differ\n".len()];
        if let Some(pos) = x.windows(b" and ".len()).position(|win| win == b" and ") {
            return DiffLine::Binaries(&x[0..pos], &x[pos + 5..]);
        }
    }

    if line.len() >= "--- x\n".len() && line.starts_with(b"--- ") {
        return DiffLine::OldFile(parse_fileinfo(line));
    }

    DiffLine::Junk(line)
}

#[test]
fn test_parse_old_file() {
    assert_eq!(parse_old_file(b""), DiffLine::Junk(b""));
    assert_eq!(
        parse_old_file(b"Binary files and differ\n"),
        DiffLine::Junk(b"Binary files and differ\n")
    );
    assert_eq!(
        parse_old_file(b"Binary files foo and bar differ\n"),
        DiffLine::Binaries(b"foo", b"bar")
    );
    assert_eq!(
        parse_old_file(b"--- x\n"),
        DiffLine::OldFile(FileInfo {
            filename: b"x",
            metadata: None
        })
    );
    assert_eq!(
        parse_old_file(b"--- foo/bar\n"),
        DiffLine::OldFile(FileInfo {
            filename: b"foo/bar",
            metadata: None
        })
    );
    assert_eq!(
        parse_old_file(b"--- foo/bar\tfoo bar\n"),
        DiffLine::OldFile(FileInfo {
            filename: b"foo/bar",
            metadata: Some(b"foo bar")
        })
    );
    assert_eq!(
        parse_old_file(b"--- foo/bar\t\n"),
        DiffLine::OldFile(FileInfo {
            filename: b"foo/bar",
            metadata: Some(b"")
        })
    );
    assert_eq!(parse_old_file(b"--- \n"), DiffLine::Junk(b"--- \n"));
}

fn parse_new_file(line: &[u8]) -> DiffLine<'_> {
    if line.len() >= b"+++ x\n".len() && line.starts_with(b"+++ ") {
        return DiffLine::NewFile(parse_fileinfo(line));
    }

    DiffLine::Junk(line)
}

#[test]
fn test_parse_new_file() {
    assert_eq!(parse_new_file(b""), DiffLine::Junk(b""));
    assert_eq!(
        parse_new_file(b"+++ x\n"),
        DiffLine::NewFile(FileInfo {
            filename: b"x",
            metadata: None
        })
    );
    assert_eq!(
        parse_new_file(b"+++ foo/bar\n"),
        DiffLine::NewFile(FileInfo {
            filename: b"foo/bar",
            metadata: None
        })
    );
    assert_eq!(
        parse_new_file(b"+++ foo/bar\tfoo bar\n"),
        DiffLine::NewFile(FileInfo {
            filename: b"foo/bar",
            metadata: Some(b"foo bar")
        })
    );
    assert_eq!(
        parse_new_file(b"+++ foo/bar\t\n"),
        DiffLine::NewFile(FileInfo {
            filename: b"foo/bar",
            metadata: Some(b"")
        })
    );
    assert_eq!(parse_new_file(b"+++ \n"), DiffLine::Junk(b"+++ \n"));
}

fn parse_hunk(line: &[u8]) -> DiffLine<'_> {
    if line.len() > b"@@ -1 +1 @@".len() && line.starts_with(b"@@ -") {
        // svn also has ## for properties
        // @@ -1,1 +1,1 @@
        // @@ -1 +1 @@

        let mut hunk = HunkInfo::default();

        let mut chunks = line[3..]
            .split(|&b| b == b' ')
            .flat_map(|chunk| parse_range(&chunk[1..]));

        if let (Some(old), Some(new)) = (chunks.next(), chunks.next()) {
            hunk.old_line_no = old.0;
            hunk.old_line_len = old.1;
            hunk.new_line_no = new.0;
            hunk.new_line_len = new.1;

            hunk.context = line.splitn(2, |&b| b == b'\t').nth(1).map(chomp);

            return DiffLine::Hunk(hunk);
        }

        return DiffLine::Junk(line);
    }

    DiffLine::Junk(line)
}

#[test]
fn test_parse_hunk() {
    assert_eq!(parse_hunk(b""), DiffLine::Junk(b""));
    assert_eq!(
        parse_hunk(b"@@ -1 +1 @@\n"),
        DiffLine::Hunk(HunkInfo {
            old_line_no: 1,
            old_line_len: 1,
            new_line_no: 1,
            new_line_len: 1,
            context: None
        })
    );
    assert_eq!(
        parse_hunk(b"@@ -12,34 +56,78 @@\n"),
        DiffLine::Hunk(HunkInfo {
            old_line_no: 12,
            old_line_len: 34,
            new_line_no: 56,
            new_line_len: 78,
            context: None
        })
    );
    assert_eq!(
        parse_hunk(b"@@ -12,34 +56,78 @@\tfoo bar\n"),
        DiffLine::Hunk(HunkInfo {
            old_line_no: 12,
            old_line_len: 34,
            new_line_no: 56,
            new_line_len: 78,
            context: Some(b"foo bar")
        })
    );
}

fn parse_delta(line: &[u8]) -> DiffLine<'_> {
    match line[0] {
        b'+' => DiffLine::Inserted(&line[1..]),
        b'-' => DiffLine::Deleted(&line[1..]),
        b'!' => DiffLine::Modified(&line[1..]),
        b' ' => DiffLine::Context(&line[1..]),
        b'\\' => DiffLine::NoNewlineAtEof,
        _ => DiffLine::Junk(line),
    }
}

#[test]
fn test_parse_delta() {
    assert_eq!(parse_delta(b"+foo\n"), DiffLine::Inserted(b"foo\n"));
    assert_eq!(parse_delta(b"-foo\n"), DiffLine::Deleted(b"foo\n"));
    assert_eq!(parse_delta(b"!foo\n"), DiffLine::Modified(b"foo\n"));
    assert_eq!(parse_delta(b" foo\n"), DiffLine::Context(b"foo\n"));
    assert_eq!(
        parse_delta(b"\\ No newline at end of file\n"),
        DiffLine::NoNewlineAtEof
    );
    assert_eq!(parse_delta(b"foo\n"), DiffLine::Junk(b"foo\n"));
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
                    DiffLine::Junk(line) => {
                        self.state = State::Junk;
                        return Some(Ok(DiffLine::Junk(line)));
                    }
                    _ => unreachable!(),
                };

                if (*old < 0 || *new < 0) || (*old == 0 && *new == 0) {
                    self.state = State::NewFile;
                }

                Some(Ok(line))
            }
        }
    }
}
