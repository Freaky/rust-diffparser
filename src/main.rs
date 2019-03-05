use diffparser::{DiffLine, DiffParser};

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
            DiffLine::Binaries(_, _) => files += 1,
            _ => (),
        }
    }

    print!(" {} files changed", files);

    if hunks > 0 {
        print!(", {} hunks", hunks);
    }

    if insert > 0 {
        print!(", {} insertions(+)", insert);
    }

    if delete > 0 {
        print!(", {} deletions(-)", delete);
    }

    if modify > 0 {
        print!(", {} modifications(!)", modify);
    }

    println!();
}

use std::env;
use std::fs::File;

fn main() {
    let filename = env::args_os().nth(1).expect("Need a path");

    let diff = File::open(filename).expect("open error");
    diffstat(BufReader::new(diff));
}
