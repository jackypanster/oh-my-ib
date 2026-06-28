//! `contract` — resolve contract details for a symbol. (card 02)

use ibapi::prelude::Contract;
use serde_json::json;

use crate::cli::ContractArgs;
use crate::config::Config;
use crate::error::AppError;

pub fn contract(cfg: &Config, args: &ContractArgs) -> Result<serde_json::Value, AppError> {
    let client = super::connect(cfg)?;
    let contract = Contract::stock(args.symbol.as_str()).build();
    let details = client
        .contract_details(&contract)
        .map_err(|e| AppError::data(format!("contract_details failed: {e}"), "contract"))?;

    if details.is_empty() {
        return Err(AppError::not_found(
            format!("no contract found for {}", args.symbol),
            "contract",
        ));
    }

    let out: Vec<_> = details
        .iter()
        .map(|d| {
            json!({
                "symbol": d.contract.symbol,
                "conid": d.contract.contract_id,
                "exchange": d.contract.exchange,
                "currency": d.contract.currency,
                "sec_type": format!("{:?}", d.contract.security_type),
                "long_name": d.long_name,
            })
        })
        .collect();
    Ok(json!({ "contracts": out }))
}
