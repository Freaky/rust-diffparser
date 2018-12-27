# diffparser

A rubbish Rust unified diff stream parser.

Seems it's fairly fast and not *entirely* broken:

```
-% wc -l huge.diff
 5065549 huge.diff
-% time target/release/diffparser huge.diff
19921 file(s) changed, 138067 hunks, 1871681 insertions(+), 1731617 deletions(-), 0 modifications(!)
0.789 real, 0.686 user, 0.102 sys;  page: 0 hard/151 soft, swap: 0, I/O: 2/0
Mem: 2752KB (138KB shared + 146KB data/stack = 285KB), VCSW: 6 IVCSW: 9
-% time diffstat huge.diff |tail -1
 19921 files changed, 1871681 insertions(+), 1731617 deletions(-)
1.460 real, 1.411 user, 0.047 sys;  page: 0 hard/1185 soft, swap: 0, I/O: 2/0
```

Not too bad for something I got working at 5am after a big dram of Ardbeg 10.
