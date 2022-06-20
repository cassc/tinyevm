use primitive_types::H160;

#[no_mangle]
pub extern "C" fn add(left: usize, right: usize) -> usize {
    left + right
}

pub fn deploy(contract_deploy_code: String, address: Option<H160>) {
    todo!()
}

pub fn invoke(contract: H160, binary: String) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
