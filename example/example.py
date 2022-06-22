from ctypes import cdll, c_uint32, c_char_p
import os
import sys
import json

libtinyevm = './target/release/libtinyevm.so'

if not os.path.exists(libtinyevm):
    print('Please build libtinyevm.so first with cargo build --release')
    sys.exit(1)

h = cdll.LoadLibrary(libtinyevm)

contract_bytecode = open('./example/C.hex').read()
owner = '0xf000000000000000000000000000000000000000'

h.deploy.argtypes = (c_char_p, c_char_p)
h.deploy.restype = c_char_p
r_address = h.deploy(contract_bytecode.encode(), owner.encode()).decode()
contract_address = '0x0803d1e6309ed01468bb1e0837567edd758bc991'

print(f'h.deploy(contract_bytecode, owner) => {r_address}, expeced: {contract_address}')

# transfer small amount of ERC tokens
tranfer_token_valid = '0xa9059cbb0000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000270f'
# transfer too large amount of ERC tokens, will cause revert
tranfer_token_invalid = '0xa9059cbb0000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000146592adc245b807743c00000'
# query balance
query_balance = '0x70a08231000000000000000000000000' + owner[2:]


h.contract_call.argtypes = (c_char_p, c_char_p, c_char_p)
h.contract_call.restype = c_char_p

# Transfer 9999 token to 0x1000000000000000000000000000000000000000
resp = h.contract_call(contract_address.encode(), owner.encode(), tranfer_token_valid.encode()).decode()
print(f'transfer(to, 9999)                 => {resp}')

resp = h.contract_call(contract_address.encode(), owner.encode(), query_balance.encode()).decode()
resp = json.loads(resp)
# Decode for balance
balance = int.from_bytes(resp[1], 'big')
print(f'balance(owner)                     => {balance}')

resp = h.contract_call(contract_address.encode(), owner.encode(), tranfer_token_invalid.encode()).decode()
resp = json.loads(resp)
# Decode for revert message
# First 32 + 32 bytes are probably contract adderss and method sig, I'm not sure
err = bytearray(resp[1][64:]).decode() 
print(f'transfer(to, HUGE_AMOUNT)          => {resp}')
print(f'                                   => {err}')



