use soroban_sdk::{Address, Env};
use soroban_sdk::token::Client as TokenClient;

use crate::errors::VaultError;

pub type CrossContractResult<T> = Result<T, VaultError>;

pub struct CrossContractClient;

impl CrossContractClient {
    pub fn token_transfer(
        e: &Env,
        token_address: &Address,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> CrossContractResult<()> {
        let token_client = TokenClient::new(e, token_address);
        token_client.transfer(from, to, &amount);
        Ok(())
    }

    pub fn token_balance(
        e: &Env,
        token_address: &Address,
        address: &Address,
    ) -> CrossContractResult<i128> {
        let token_client = TokenClient::new(e, token_address);
        Ok(token_client.balance(address))
    }

    pub fn validate_contract_exists(
        e: &Env,
        contract_address: &Address,
    ) -> CrossContractResult<()> {
        if contract_address == &e.current_contract_address() {
            return Err(VaultError::InvalidAddress);
        }
        Ok(())
    }
}
