from ctypes import cdll, c_uint32, c_char_p
import os
import sys
libtinyevm = './target/release/libtinyevm.so'

if not os.path.exists(libtinyevm):
    print('Please build libtinyevm.so first with cargo build --release')
    sys.exit(1)

h = cdll.LoadLibrary(libtinyevm)

contract_bytecode = open('./example/C.hex').read()
owner = '0xf000000000000000000000000000000000000000'

h.deploy.argtypes = (c_char_p, c_char_p)
h.deploy.restype = c_char_p
contract_address = h.deploy(contract_bytecode.encode(), owner.encode()).decode()

print(f'h.deploy(contract_bytecode, owner) => {contract_address}')
