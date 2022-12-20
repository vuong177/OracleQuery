use std::{ops::Deref, vec};

use cosmos_sdk_proto::Any;
use cosmos_sdk_proto::ibc::applications::interchain_accounts::v1::CosmosTx;
use cosmos_sdk_proto::traits::MessageExt;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coin, to_binary, Addr, BankMsg, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, QueryRequest, Response, StdError, StdResult, Storage, Uint128, Empty,to_vec
};

use cosmos_sdk_proto::cosmos::bank::v1beta1::{QueryBalanceRequest, QueryBalanceResponse};
use prost::{Message};

use cw2::set_contract_version;

use crate::querier::{UltraQuerier, self};
use crate::{error::ContractError, state::Rate};
use juno_stable::oracle_querier::{
    ExchangeRateResponse, ExecuteMsg, InstantiateMsg, OracleQuery, QueryMsg, UltraQuery,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:active-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const NATIVE_JUNO_DENOM: &str = "ujuno";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::GetExchangeRate { denom } => get_exchange_rate(deps, denom),
    }
}

pub fn get_exchange_rate(deps: DepsMut, denom: String) -> Result<Response, ContractError> {
    let query: UltraQuery = UltraQuery::Oracle(OracleQuery::ExchangeRate {
        denom: denom.clone(),
    });
    let request: QueryRequest<UltraQuery> = UltraQuery::into(query);
    let querier: QuerierWrapper<UltraQuery> =
        QuerierWrapper::<UltraQuery>::new(deps.querier.deref());
    let exchangerate: ExchangeRateResponse = querier.query(&request)?;
    Rate.save(deps.storage, denom.clone(), &exchangerate.rate)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "get_rate"),
        attr("denom", denom),
        attr("rate", exchangerate.rate.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ExchangeRate { denom } => to_binary(&query_exchange_rate_stargate(deps, denom)?),
    }
}

pub fn query_exchange_rate(deps: Deps, denom: String) -> StdResult<Decimal> {
    let rate = Rate.load(deps.storage, denom)?;
    Ok(rate)
}


pub fn query_exchange_rate_stargate(deps: Deps, denom: String) -> StdResult<String> {
    let query_request = QueryBalanceRequest {
        address : "juno1eqtu0fkge4gfsr8ghp27pjuxps2yx3lqpeqad8".to_string(),
        denom,
    };

    let vecu8_query_request = query_request.encode_to_vec();
    let data =Binary::from(vecu8_query_request);

    let query_request:QueryRequest<Empty> = QueryRequest::Stargate {
        path: "/cosmos.bank.v1beta1.Query/AllBalances".to_string(),
        data : data,
    };

    let raw = to_vec(&query_request).map_err(|serialize_err| {
        StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
    }).unwrap();

    let response1 = deps.querier.raw_query(&raw).unwrap().unwrap();
    let a = response1.as_slice();

    let response  = QueryBalanceResponse::decode(a).unwrap();

    let a = response.balance.unwrap().amount;
    return Ok(a);
}