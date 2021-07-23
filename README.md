## serde_bson

Originally implemented as a workaround to the `bson` crate cloning every value it
comes across and it's looking like it shows significant improvement across the board
for serialisation (~80% improvement).

```
borrowed: mongodb's bson  time:   [1.1160 us 1.1171 us 1.1183 us]
Found 2 outliers among 100 measurements (2.00%)
  2 (2.00%) high mild

borrowed: serde_bson      time:   [201.99 ns 202.17 ns 202.38 ns]                                 
Found 10 outliers among 100 measurements (10.00%)
  4 (4.00%) low mild
  4 (4.00%) high mild
  2 (2.00%) high severe
```

Even on owned data it shows a significant improvement:

```
owned: mongodb's bson	time:   [1.0740 us 1.0762 us 1.0794 us]                                   
Found 6 outliers among 100 measurements (6.00%)
  4 (4.00%) low mild
  1 (1.00%) high mild
  1 (1.00%) high severe

owned: serde_bson	time:   [209.67 ns 210.18 ns 211.06 ns]                              
Found 9 outliers among 100 measurements (9.00%)
  5 (5.00%) high mild
  4 (4.00%) high severe
```

There's a few pieces missing such as arrays and nested documents but they're not
too difficult to add, it's just that it's 2:38am and I've smashed this out in an
hour.

Pull requests welcome as always.
