
use diffparser::{DiffParser, DiffLine};

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
