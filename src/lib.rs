extern crate lazy_static;
extern crate libc;

use evm::backend::{MemoryAccount, MemoryBackend, MemoryVicinity};
use evm::executor::stack::{
    MemoryStackState, MemoryStackSubstate, PrecompileFailure, PrecompileOutput, PrecompileSet,
    StackExecutor, StackState, StackSubstateMetadata,
};
use evm::{Capture, Config, Context, ExitReason, Handler};
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

lazy_static! {
    static ref OWNER: H160 = H160::from_str("0xf000000000000000000000000000000000000000").unwrap();
    static ref INITIAL_STATE: BTreeMap<H160, MemoryAccount> = btreemap! {
        OWNER.to_owned() => MemoryAccount {
            nonce: U256::zero(),
            balance: U256::from(66_666_666u64),
            storage: BTreeMap::new(),
            code: Vec::new(),
        }
    };
    static ref EVM_STATES: Arc<Mutex<BTreeMap<H160, MemoryAccount>>> =
        Arc::new(Mutex::new(INITIAL_STATE.to_owned()));
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
    let address = deploy_helper(INITIAL_STATE.to_owned(), owner, contract_bytecode).unwrap();
    let address = address.encode_hex::<String>();
    let address = CString::new(address).unwrap();
    // let address = address.as_c_str().as_ptr();
    address.into_raw()
}

#[no_mangle]
pub extern "C" fn invoke(contract: H160, binary: String) -> String {
    todo!()
}

/// Starting a EVM executor with the provided `initial_states` and
/// deploy the contract for the `owner`. Returns the address of the
/// contract if deployment is success.
fn deploy_helper(
    initial_states: BTreeMap<H160, MemoryAccount>,
    owner: H160,
    contract_bytecode: Vec<u8>,
) -> Result<H160> {
    // Configure EVM executor and set initial state
    let config = Config::istanbul();
    let vicinity = MemoryVicinity {
        gas_price: U256::zero(),
        origin: H160::default(),
        chain_id: U256::one(),
        block_hashes: Vec::new(),
        block_number: Default::default(),
        block_coinbase: Default::default(),
        block_timestamp: Default::default(),
        block_difficulty: U256::one(),
        block_gas_limit: U256::zero(),
        block_base_fee_per_gas: U256::zero(),
    };

    // Create executor
    let backend = MemoryBackend::new(&vicinity, initial_states);
    let gas_limit = u64::MAX; // max gas limit allowed in this execution
    let metadata = StackSubstateMetadata::new(gas_limit, &config);
    let state = MemoryStackState::new(metadata, &backend);
    let precompiles = BTreeMap::new();
    let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);

    let reason = executor.create(
        owner,
        evm::CreateScheme::Legacy { caller: owner },
        U256::default(),
        contract_bytecode,
        None,
    );

    if let Capture::Exit((_reason, address, _return_data)) = reason {
        println!("Contract deployed to adderss {address:?} ");
        let contract_address = address.context("Missing contract address, deployment failed")?;
        Ok(contract_address)
    } else {
        return Err(eyre!("Contract deploy failed, {reason:#?}!"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
