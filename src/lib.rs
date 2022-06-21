#![feature(test)]

extern crate lazy_static;
extern crate libc;

use evm::backend::{MemoryAccount, MemoryBackend, MemoryVicinity};
use evm::executor::stack::{
    MemoryStackState, MemoryStackSubstate, PrecompileFailure, PrecompileOutput, PrecompileSet,
    StackExecutor, StackState, StackSubstateMetadata,
};
use evm::{Capture, Config, Context, CreateScheme, ExitReason, Handler};
use eyre::{eyre, ContextCompat, Result};
use hex::ToHex;
use lazy_static::lazy_static;
use libc::c_char;
use maplit::btreemap;
use primitive_types::{H160, H256, U256};
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

type StaticStackExecutor =
    StackExecutor<'static, 'static, MemoryStackState<'static, 'static, MemoryBackend<'static>>, ()>;

const MAX_GAS: u64 = 1_000_000_000_000_000;

lazy_static! {
    pub static ref OWNER: H160 =
        H160::from_str("0xf000000000000000000000000000000000000000").unwrap();
    pub static ref INITIAL_STATE: BTreeMap<H160, MemoryAccount> = btreemap! {
        OWNER.to_owned() => MemoryAccount {
            nonce: U256::zero(),
            balance: U256::from(1_000_000_000_000u64),
            storage: BTreeMap::new(),
            code: Vec::new(),
        }
    };
    static ref ISTANBUL: Config = Config::istanbul();
    static ref VICINITY: MemoryVicinity = MemoryVicinity {
        gas_price: U256::zero(),
        origin: H160::zero(),
        chain_id: U256::zero(),
        block_hashes: Vec::new(),
        block_number: U256::zero(),
        block_coinbase: H160::zero(),
        block_timestamp: U256::zero(),
        block_difficulty: U256::zero(),
        block_gas_limit: U256::zero(),
        block_base_fee_per_gas: U256::zero(),
    };
    static ref EVM_BACKEND: MemoryBackend<'static> = {
        let initial_states = INITIAL_STATE.to_owned();
        MemoryBackend::new(&VICINITY, initial_states)
    };
    static ref EVM_EXECUTOR: Arc<Mutex<StaticStackExecutor>> = {
        let metadata = StackSubstateMetadata::new(MAX_GAS, &ISTANBUL);
        let state = MemoryStackState::new(metadata, &*EVM_BACKEND);
        Arc::new(Mutex::new(StackExecutor::new_with_precompiles(
            state,
            &ISTANBUL,
            &(),
        )))
    };
}

/// Deploy a contract using contract deploy binary as hex
/// `contract_deploy_code` using the account `owner`. Returns the
/// address of the deployed contract.
#[no_mangle]
pub unsafe extern "C" fn deploy(
    contract_deploy_code: *const c_char,
    owner: *const c_char,
) -> *mut c_char {
    let owner = CStr::from_ptr(owner);
    let owner = owner.to_owned().into_string().unwrap();
    let contract_deploy_code = CStr::from_ptr(contract_deploy_code);
    let contract_deploy_code = contract_deploy_code.to_owned().into_string().unwrap();
    let owner = H160::from_str(&owner).unwrap();

    let contract_bytecode = hex::decode(&contract_deploy_code).unwrap();
    let address = deploy_helper(None, H256::zero(), owner, contract_bytecode).unwrap();
    let address = address.encode_hex::<String>();
    let address = CString::new(address).unwrap();
    // let address = address.as_c_str().as_ptr();
    address.into_raw()
}

#[no_mangle]
pub extern "C" fn invoke() -> String {
    todo!()
}

fn contract_call(contract: H160, owner: H160, data: Vec<u8>) -> (ExitReason, Vec<u8>) {
    let mut executor = EVM_EXECUTOR.lock().unwrap();
    executor.transact_call(owner, contract, U256::zero(), data, u64::MAX, Vec::new())
}

/// Starting a EVM executor with an optional `initial_states` and
/// deploy the contract for the `owner`. Returns the address of the
/// contract if deployment is success.
fn deploy_helper(
    _initial_states: Option<BTreeMap<H160, MemoryAccount>>,
    salt: H256,
    owner: H160,
    contract_bytecode: Vec<u8>,
) -> Result<H160> {
    let mut executor = EVM_EXECUTOR.lock().unwrap();
    let reason = executor.transact_create2(
        owner,                     // caller
        U256::zero(),              // value
        contract_bytecode.clone(), // init code
        salt,                      // salt
        u64::MAX,                  // gas limit
        vec![],                    // access list
    );

    println!("Contract deployed response {reason:#?} ");
    if let (ExitReason::Succeed(_), _) = reason {
        // This part just calculate the address of the contract, which is:
        // https://ethereum.stackexchange.com/questions/760/how-is-the-address-of-an-ethereum-contract-computed
        // keccak256( 0xff ++ senderAddress ++ salt ++ keccak256(init_code))[12:]
        let code_hash = H256::from_slice(Keccak256::digest(&contract_bytecode).as_slice());
        let caller = owner.clone();
        let contract = executor.create_address(CreateScheme::Create2 {
            caller,
            code_hash,
            salt,
        });
        println!("Contract deployed to adderss {contract:?} ");
        Ok(contract)
    } else {
        return Err(eyre!("Contract deploy failed, {reason:#?}!"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;
    use test::Bencher;

    lazy_static! {
        static ref CONTRACT_ADDRESS: H160 =
            H160::from_str("0x04f21ff87c9e4930e6830e5eb25414f999bdf409").unwrap();
        // balanceOf(owner)
        static ref ERC20_BALANCE_BIN: Vec<u8> =
            hex::decode("70a08231000000000000000000000000f000000000000000000000000000000000000000")
            .unwrap();
        // transfer(to, value) with value < owner value
        static ref ERC20_TRANSFER_BIN: Vec<u8> = hex::decode("").unwrap();
        // transfer(to, value) with value > owner value
        static ref ERC20_REVERT_BIN: Vec<u8> = hex::decode("").unwrap();
        // total supply of the tokens as defined in contract C, also equals initial token balance of owner
        static ref TOKEN_SUPPLY: U256 = U256::from_dec_str("10000000000000000000000").unwrap();
    }

    fn t_deploy() {
        let owner = OWNER.to_owned();
        let bytecode = include_str!("../example/C.hex");
        let bytecode = hex::decode(bytecode).unwrap();

        let address = deploy_helper(None, H256::zero(), owner, bytecode.clone()).unwrap();
        assert_eq!(*CONTRACT_ADDRESS, address, "Deployed address {address:#?}");
    }

    fn t_erc20_balance_query() {
        let reason = contract_call(*CONTRACT_ADDRESS, *OWNER, ERC20_BALANCE_BIN.to_owned());
        assert!(matches!(reason.0, ExitReason::Succeed(_)));
        let balance = {
            if let (ExitReason::Succeed(_), data) = reason {
                U256::from_big_endian(data.as_slice())
            } else {
                // branch not reachable
                U256::zero()
            }
        };

        assert_eq!(*TOKEN_SUPPLY, balance);
    }

    fn t_transfer() {
        let reason = contract_call(*CONTRACT_ADDRESS, *OWNER, ERC20_TRANSFER_BIN.to_owned());
    }

    #[test]
    fn test_deploy_query_balance() {
        t_deploy();
        t_erc20_balance_query();
    }

    #[bench]
    fn bench_deploy(b: &mut Bencher) {
        let owner = OWNER.to_owned();
        let bytecode = include_str!("../example/C.hex");
        let bytecode = hex::decode(bytecode).unwrap();
        b.iter(|| deploy_helper(None, H256::random(), owner, bytecode.clone()));
    }
}
