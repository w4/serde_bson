## serde_bson

Originally implemented as a workaround to the `bson` crate cloning every value it
comes across. The `bson` crate has since improved in this aspect, however this
clean room implementation of the spec still shows significant speedup in both
serialisation and deserialisation.

```
deserialize: mongodb's bson
                        time:   [867.32 ns 867.62 ns 867.97 ns]

deserialize: serde_bson time:   [468.41 ns 470.12 ns 472.06 ns]

serialize: mongodb's bson
                        time:   [684.01 ns 686.48 ns 689.57 ns]

serialize: serde_bson   time:   [136.42 ns 136.86 ns 137.36 ns]
```
