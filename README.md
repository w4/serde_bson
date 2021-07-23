## serde_bson

Originally implemented as a workaround to the `bson` crate cloning every value it
comes across and it's looking like it shows significant improvement across the board
for serialisation (~80% improvement).

```
mongodb's bson  time:   [1.1160 us 1.1171 us 1.1183 us]
Found 2 outliers among 100 measurements (2.00%)
  2 (2.00%) high mild

serde_bson      time:   [201.99 ns 202.17 ns 202.38 ns]                                 
Found 10 outliers among 100 measurements (10.00%)
  4 (4.00%) low mild
  4 (4.00%) high mild
  2 (2.00%) high severe
```

There's a few pieces missing such as arrays and nested documents but they're not
too difficult to add, it's just that it's 2:38am and I've smashed this out in an
hour.

Pull requests welcome as always.