#![feature(test)]
#![feature(assert_matches)]
#![feature(local_key_cell_methods)]
extern crate lazy_static;
extern crate libc;
use evm::backend::{MemoryAccount, MemoryBackend, MemoryVicinity};
use evm::executor::stack::{MemoryStackState, StackExecutor, StackSubstateMetadata};
use evm::{Config, CreateScheme, ExitReason};
use eyre::{eyre, Result};
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
use tracing::{debug, info};

type StaticStackExecutor<'a> =
    StackExecutor<'a, 'a, MemoryStackState<'a, 'a, MemoryBackend<'a>>, ()>;

const MAX_GAS: u64 = 1_000_000_000_000_000;

// thread_local! {
//     static EVM_EXECUTOR: Cell<Option<StaticStackExecutor>> = {
//         let metadata = StackSubstateMetadata::new(MAX_GAS, &ISTANBUL);
//         let state = MemoryStackState::new(metadata, &*EVM_BACKEND);
//         Cell::new(Some(StackExecutor::new_with_precompiles(
//             state,
//             &ISTANBUL,
//             &(),
//         )))
//     };
// }

lazy_static! {
    /// Default sender of all transactions
    pub static ref OWNER: H160 =
        H160::from_str("0xf000000000000000000000000000000000000000").unwrap();
    /// Initial states of the EVM, gives OWNER 1e12 ethers
    pub static ref INITIAL_STATE: BTreeMap<H160, MemoryAccount> = btreemap! {
        OWNER.to_owned() => MemoryAccount {
            nonce: U256::zero(),
            balance: U256::from(1_000_000_000_000u64),
            storage: btreemap!{
                H256::random() => H256::random()
            },
            code: Vec::new(),
        }
    };
    /// Using ISTANBUL fork
    static ref ISTANBUL: Config = Config::istanbul();
    /// Vicinity setting does not matter for our usage
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
    /// EVM backend from vicinity initial states
    static ref EVM_BACKEND: MemoryBackend<'static> = {
        let initial_states = INITIAL_STATE.to_owned();
        MemoryBackend::new(&VICINITY, initial_states)
    };
    /// Thread safe EVM executor
    static ref EVM_EXECUTOR: Arc<Mutex<StaticStackExecutor<'static>>> = {
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
    // let mut exe = make_executor();
    let mut exe = EVM_EXECUTOR.lock().unwrap();
    let address = deploy_helper(&mut exe, None, H256::zero(), owner, contract_bytecode).unwrap();
    let address = format!("0x{}", address.encode_hex::<String>());
    let address = CString::new(address).unwrap();
    // let address = address.as_c_str().as_ptr();
    address.into_raw()
}

#[no_mangle]
pub extern "C" fn contract_call(
    contract: *const c_char,
    sender: *const c_char,
    data: *const c_char,
) -> *mut c_char {
    let sender = unsafe { CStr::from_ptr(sender).to_owned().into_string().unwrap() };
    let contract = unsafe { CStr::from_ptr(contract).to_owned().into_string().unwrap() };
    let data = unsafe { CStr::from_ptr(data).to_owned().into_string().unwrap() };
    info!("contract_call: {} {} {}", contract, sender, data);

    let sender = H160::from_str(&sender).unwrap();
    let contract = H160::from_str(&contract).unwrap();
    let data = {
        if data.starts_with("0x") || data.starts_with("0X") {
            let data = data[2..].to_owned();
            hex::decode(data).unwrap()
        } else {
            hex::decode(data).unwrap()
        }
    };
    // let mut exe = make_executor();

    let mut exe = EVM_EXECUTOR.lock().unwrap();
    let resp = contract_call_helper(&mut exe, contract, sender, data);
    let s = serde_json::to_string(&resp).unwrap();
    let s = CString::new(s).unwrap();
    s.into_raw()
}

/// Excecute a contract method with inital states
#[no_mangle]
pub extern "C" fn contract_call_with_initial_states(
    initial_states: *const c_char,
    contract: *const c_char,
    sender: *const c_char,
    data: *const c_char,
) -> *mut c_char {
    let initial_states = unsafe {
        CStr::from_ptr(initial_states)
            .to_owned()
            .into_string()
            .unwrap()
    };
    let initial_states: BTreeMap<H160, MemoryAccount> =
        serde_json::from_str(&initial_states).unwrap();

    debug!("initial-states: {initial_states:?}");

    let sender = unsafe { CStr::from_ptr(sender).to_owned().into_string().unwrap() };
    let contract = unsafe { CStr::from_ptr(contract).to_owned().into_string().unwrap() };
    let data = unsafe { CStr::from_ptr(data).to_owned().into_string().unwrap() };
    info!("contract_call: {} {} {}", contract, sender, data);

    let sender = H160::from_str(&sender).unwrap();
    let contract = H160::from_str(&contract).unwrap();
    let data = {
        if data.starts_with("0x") || data.starts_with("0X") {
            let data = data[2..].to_owned();
            hex::decode(data).unwrap()
        } else {
            hex::decode(data).unwrap()
        }
    };
    let backend = MemoryBackend::new(&VICINITY, initial_states);
    let mut exe = make_executor_with_initial_states(&backend);
    let resp = contract_call_helper(&mut exe, contract, sender, data);
    let s = serde_json::to_string(&resp).unwrap();
    let s = CString::new(s).unwrap();
    s.into_raw()
}

fn contract_call_helper(
    executor: &mut StaticStackExecutor,
    contract: H160,
    sender: H160,
    data: Vec<u8>,
) -> (ExitReason, Vec<u8>) {
    // let mut executor: StaticStackExecutor = EVM_EXECUTOR.take().unwrap();
    // let mut executor = make_executor();
    let r = executor.transact_call(sender, contract, U256::zero(), data, u64::MAX, Vec::new());
    // EVM_EXECUTOR.set(Some(executor));
    r
}

/// Starting a EVM executor with an optional `initial_states` and
/// deploy the contract for the `owner`. Returns the address of the
/// contract if deployment is success.
fn deploy_helper(
    executor: &mut StaticStackExecutor,
    _initial_states: Option<BTreeMap<H160, MemoryAccount>>,
    salt: H256,
    owner: H160,
    contract_bytecode: Vec<u8>,
) -> Result<H160> {
    let reason = executor.transact_create2(
        owner,                     // caller
        U256::zero(),              // value
        contract_bytecode.clone(), // init code
        salt,                      // salt
        u64::MAX,                  // gas limit
        vec![],                    // access list
    );

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
        info!("Contract deployed to adderss {contract:?} ");
        Ok(contract)
    } else {
        Err(eyre!("Contract deploy failed, {reason:#?}!"))
    }
}

#[allow(dead_code)]
/// Create a new EVM executor
fn make_executor() -> StaticStackExecutor<'static> {
    let metadata = StackSubstateMetadata::new(MAX_GAS, &ISTANBUL);
    let state = MemoryStackState::new(metadata, &*EVM_BACKEND);
    StackExecutor::new_with_precompiles(state, &ISTANBUL, &())
}

fn make_executor_with_initial_states<'a>(
    backend: &'a MemoryBackend<'a>,
) -> StaticStackExecutor<'a> {
    let metadata = StackSubstateMetadata::new(MAX_GAS, &ISTANBUL);
    let state = MemoryStackState::new(metadata, backend);
    StackExecutor::new_with_precompiles(state, &ISTANBUL, &())
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::*;
    extern crate test;
    use test::Bencher;
    // balanceOf(owner)
    const ERC20_BALANCE_BIN_PREFIX: &'static str = "70a08231000000000000000000000000";
    const TRANSFER_TOKEN_VALUE: u64 = 9999;

    lazy_static! {
        static ref CONTRACT_ADDRESS: H160 =
            H160::from_str("0x0803d1e6309ed01468bb1e0837567edd758bc991").unwrap();
        // Target address to receive some ERC20 tokens
        static ref TO_ADDRESS: H160 =
            H160::from_str("0x1000000000000000000000000000000000000000").unwrap();
        // transfer(to, value) with value = TRANSFER_TOKEN_VALUE to TO_ADDRESS
        static ref ERC20_TRANSFER_BIN: Vec<u8> = hex::decode("a9059cbb0000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000270f").unwrap();
        // transfer(to, value) with value > owner value
        static ref ERC20_REVERT_BIN: Vec<u8> = hex::decode("a9059cbb0000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000146592adc245b807743c00000").unwrap();
        // total supply of the tokens as defined in contract C, also equals initial token balance of owner
        static ref TOKEN_SUPPLY: U256 = U256::from_dec_str("10000000000000000000000").unwrap();
    }

    fn t_deploy(executor: &mut StaticStackExecutor) {
        let owner = OWNER.to_owned();
        let bytecode = include_str!("../example/C.hex");
        let bytecode = hex::decode(bytecode).unwrap();

        let address = deploy_helper(executor, None, H256::zero(), owner, bytecode.clone()).unwrap();

        println!(
            "Start states: {}",
            serde_json::to_string(&*INITIAL_STATE).unwrap()
        );

        assert_eq!(*CONTRACT_ADDRESS, address, "Deployed address {address:#?}");
    }

    fn t_erc20_balance_query(exe: &mut StaticStackExecutor, address: H160, expected_balance: U256) {
        let data = format!(
            "{}{}",
            ERC20_BALANCE_BIN_PREFIX,
            address.encode_hex::<String>()
        );
        let data = hex::decode(data).unwrap();
        let reason = contract_call_helper(exe, *CONTRACT_ADDRESS, *OWNER, data);
        assert!(matches!(reason.0, ExitReason::Succeed(_)));
        let balance = {
            if let (ExitReason::Succeed(_), data) = reason {
                U256::from_big_endian(data.as_slice())
            } else {
                // branch not reachable
                U256::zero()
            }
        };

        assert_eq!(expected_balance, balance);
    }

    #[test]
    fn test_contract_deploy_transfer_query() {
        let mut exe = make_executor();
        t_deploy(&mut exe);

        t_erc20_balance_query(&mut exe, *OWNER, *TOKEN_SUPPLY);
        t_erc20_balance_query(&mut exe, *TO_ADDRESS, U256::zero());

        for _ in 0..2 {
            let result = contract_call_helper(
                &mut exe,
                *CONTRACT_ADDRESS,
                *OWNER,
                ERC20_TRANSFER_BIN.to_owned(),
            );
            assert_matches!(result, (ExitReason::Succeed(_), _));
        }

        t_erc20_balance_query(&mut exe, *OWNER, *TOKEN_SUPPLY - 2 * TRANSFER_TOKEN_VALUE);
        t_erc20_balance_query(
            &mut exe,
            *TO_ADDRESS,
            U256::zero() + 2 * TRANSFER_TOKEN_VALUE,
        );
    }

    #[test]
    fn test_contract_method_revert() {
        let mut exe = make_executor();
        t_deploy(&mut exe);
        let result = contract_call_helper(
            &mut exe,
            *CONTRACT_ADDRESS,
            *OWNER,
            ERC20_REVERT_BIN.to_owned(),
        );
        println!("T resp: {:?}", result);
        assert_matches!(result, (ExitReason::Revert(_), _));
    }

    #[bench]
    fn bench_token_transfer(b: &mut Bencher) {
        let mut exe = make_executor();
        t_deploy(&mut exe);

        let mut owner_balance = *TOKEN_SUPPLY;
        b.iter(|| {
            let result = contract_call_helper(
                &mut exe,
                *CONTRACT_ADDRESS,
                *OWNER,
                ERC20_TRANSFER_BIN.to_owned(),
            );
            assert_matches!(result, (ExitReason::Succeed(_), _));
            owner_balance = owner_balance
                .checked_sub(U256::from(TRANSFER_TOKEN_VALUE))
                .unwrap();
            t_erc20_balance_query(&mut exe, *OWNER, owner_balance);
        });
    }

    #[bench]
    fn bench_deploy(b: &mut Bencher) {
        let owner = OWNER.to_owned();
        let bytecode = include_str!("../example/C.hex");
        let bytecode = hex::decode(bytecode).unwrap();
        b.iter(|| {
            let mut exe = make_executor();
            deploy_helper(&mut exe, None, H256::random(), owner, bytecode.clone())
        });
    }
}
