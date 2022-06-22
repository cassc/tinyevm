# py-evm-executor
Executor EVM bytecode from Python


- To build dynamic library, run 

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

- You can find the example usage in Python at `example/example.py`

Example files:

``` text
example
├── example/C.hex      # Compiled contract deploy bytecode as hex
└── example/example.py # Sample code in Python 
```

The contract `C` used in the above example is compiled from [data_structures.sol](https://github.com/cassc/evm-play/tree/main/contracts).
