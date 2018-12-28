# diffparser

Basic streaming unified diff parser for Rust.

It's fairly fast:

```
-% wc -l huge.diff
 5065549 huge.diff

-% time target/release/diffparser huge.diff
 19921 files changed, 138067 hunks, 1871681 insertions(+), 1731617 deletions(-)
0.918 real, 0.767 user, 0.150 sys;  page: 0 hard/151 soft, swap: 0, I/O: 2/0
Mem: 2752KB (138KB shared + 146KB data/stack = 284KB), VCSW: 6 IVCSW: 11

-% time diffstat -s huge.diff
 19921 files changed, 1871681 insertions(+), 1731617 deletions(-)
1.443 real, 1.403 user, 0.039 sys;  page: 0 hard/1180 soft, swap: 0, I/O: 2/0
Mem: 6488KB (31KB shared + 138KB data/stack = 170KB), VCSW: 9 IVCSW: 17

-% wc -l 86d7f5d.diff
 14397595 86d7f5d.diff

-% time target/release/diffparser 86d7f5d.diff
 31912 files changed, 31804 hunks, 14206216 insertions(+)
1.905 real, 1.668 user, 0.236 sys;  page: 0 hard/144 soft, swap: 0, I/O: 2/0
Mem: 2724KB (140KB shared + 149KB data/stack = 290KB), VCSW: 12 IVCSW: 22

-% time diffstat -s 86d7f5d.diff
 31912 files changed, 14206216 insertions(+)
2.823 real, 2.727 user, 0.095 sys;  page: 0 hard/1602 soft, swap: 0, I/O: 2/0
Mem: 8196KB (31KB shared + 138KB data/stack = 170KB), VCSW: 16 IVCSW: 32
```

It's also poorly-tested, has an API I've barely thought through, and was largely
written when I was drunk.  So, um, maybe use [unidiff] for now?


[unidiff]: https://crates.io/crates/unidiff
