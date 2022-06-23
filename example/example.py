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
token_supply = 1_000_000_000_000

# transfer 9999 of ERC tokens
tranfer_token_valid = '0xa9059cbb0000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000270f'
# transfer too large amount of ERC tokens, will cause revert
tranfer_token_invalid = '0xa9059cbb0000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000146592adc245b807743c00000'
# query balance
query_balance = '0x70a08231000000000000000000000000' + owner[2:]

print('=' * 80)
print('Invoke contract with persistent states')
################################################################################
# All functions exception those with `*_with_initial_states` persists states in thread-safe manner
# Deploy contract
h.deploy.argtypes = (c_char_p, c_char_p)
h.deploy.restype = c_char_p
r_address = h.deploy(contract_bytecode.encode(), owner.encode()).decode()
contract_address = '0x0803d1e6309ed01468bb1e0837567edd758bc991'

print(f'h.deploy(contract_bytecode, owner) => {r_address}, expeced: {contract_address}')

################################################################################
# Transfer ERC20 tokens
h.contract_call.argtypes = (c_char_p, c_char_p, c_char_p)
h.contract_call.restype = c_char_p

# Transfer 9999 token to 0x1000000000000000000000000000000000000000
resp = h.contract_call(contract_address.encode(), owner.encode(), tranfer_token_valid.encode()).decode()
print(f'transfer(to, 9999)                 => {resp}')

################################################################################
# Query ERC20 token balance
resp = h.contract_call(contract_address.encode(), owner.encode(), query_balance.encode()).decode()
resp = json.loads(resp)
# Decode for balance
balance = int.from_bytes(resp[1], 'big')
print(f'balance(owner)                     => {balance}')

################################################################################
# Trigger transaction revert by trying to send amount larger than user balance
resp = h.contract_call(contract_address.encode(), owner.encode(), tranfer_token_invalid.encode()).decode()
resp = json.loads(resp)
# Decode for revert message
err = bytearray(resp[1][64:]).decode() 
print(f'transfer(to, HUGE_AMOUNT)          => {resp}')
print(f'                                   => {err}')

################################################################################
# Invoke contract methods with some initial states
# Note this function uses the input state, the state is DROPPED when the call ends
print('=' * 80)
print('Invoke contract with initial states')
deployed_contract = list(bytes.fromhex(open('./example/C_deployed.hex').read()))
another_contract_address = '0xabcd000000000000000000000000000000000000'
# storages are idx (U256) -> U256 values, need to pad key/values first before sending 
k = '0x{:0>64}'.format('1fa2419a39ea8c5f47686d559ebf8d6cc06a4d9038d8cd4fa0fb86dfdbd12e85')
v = '0x{:0>64}'.format(hex(9898)[2:])
initial_states = {another_contract_address: {"nonce":"0x1",
                                             "balance":"0x0",
                                             "storage":{k: v},
                                             "code": deployed_contract},
                  owner: {"nonce":"0x0",
                          "balance":"0xe8d4a51000",
                          "storage":{},
                          "code": []}}
initial_states = json.dumps(initial_states)

h.contract_call_with_initial_states.argtypes = (c_char_p, c_char_p, c_char_p, c_char_p)
h.contract_call_with_initial_states.restype = c_char_p
resp = h.contract_call_with_initial_states(initial_states.encode(), another_contract_address.encode(), owner.encode(), query_balance.encode()).decode()
resp = json.loads(resp)
balance = int.from_bytes(resp[1], 'big')
print(f'balance(owner)                     => {balance}')

resp = h.contract_call_with_initial_states(initial_states.encode(), another_contract_address.encode(), owner.encode(), tranfer_token_valid.encode()).decode()
resp = json.loads(resp)
err = bytearray(resp[1][64:]).decode() 
print(f'transfer(to, 9999)                 => {resp}')
print(f'                                   => {err}')
