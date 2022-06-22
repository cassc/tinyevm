# tinyevm
Executor EVM bytecode from Python

## Build dynamically linked library

Build a dynamic library with the following command, the dynamic link file will be generated at `target/release/libtinyevm.so`.

``` bash
make build
```

- For unit test, run

``` bash
make test
```

- For benchmark test, run

``` bash
make bench
```

## Sample usage

You can find example usage in Python at `example/example.py`

Example files:

``` text
example
├── example/C.hex      # Compiled contract deploy bytecode as hex
└── example/example.py # Sample code in Python 
```

The contract `C` used in this example is compiled from [data_structures.sol](https://github.com/cassc/evm-play/tree/main/contracts).
