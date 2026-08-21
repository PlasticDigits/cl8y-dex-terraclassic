use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BlockResponse {
    pub block_id: Option<BlockId>,
    pub block: Block,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockId {
    pub hash: String,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
}

#[derive(Debug, Deserialize)]
pub struct BlockHeader {
    pub height: String,
    pub time: String,
}

#[derive(Debug, Deserialize)]
pub struct TxSearchResponse {
    pub tx_responses: Option<Vec<TxResponse>>,
    pub pagination: Option<Pagination>,
    /// SDK 0.50+ moved the result count to a top-level `total` and returns `pagination: null`.
    /// Older (v3.x) LCDs only populate `pagination.total`; read this first, fall back to that.
    #[serde(default)]
    pub total: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub total: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TxResponse {
    pub height: String,
    pub txhash: String,
    pub logs: Option<Vec<TxLog>>,
    pub timestamp: Option<String>,
    pub events: Option<Vec<Event>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TxLog {
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct SmartQueryResponse {
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cw20TokenInfoResponse {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PairInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: String,
    pub liquidity_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
}

/// Factory `QueryMsg::Pair { asset_infos }` response.
#[derive(Debug, Clone, Deserialize)]
pub struct FactoryPairResponse {
    pub pair: PairInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeeConfigResponse {
    pub fee_config: FeeConfigInner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeeConfigInner {
    pub fee_bps: u16,
    pub treasury: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HooksResponse {
    pub hooks: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PairCountResponse {
    pub count: u64,
}

/// LCD `GET /cosmwasm/wasm/v1/contract/{addr}` envelope (GitLab #585).
#[derive(Debug, Clone, Deserialize)]
pub struct WasmContractInfoEnvelope {
    pub contract_info: WasmContractInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WasmContractInfo {
    /// LCD may emit a JSON number or a decimal string.
    pub code_id: serde_json::Value,
}

impl WasmContractInfo {
    pub fn code_id_u64(&self) -> Option<u64> {
        match &self.code_id {
            serde_json::Value::Number(n) => n.as_u64(),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_id_parses_string_or_number() {
        let n: WasmContractInfo =
            serde_json::from_value(serde_json::json!({ "code_id": 10184 })).unwrap();
        assert_eq!(n.code_id_u64(), Some(10184));
        let s: WasmContractInfo =
            serde_json::from_value(serde_json::json!({ "code_id": "6036" })).unwrap();
        assert_eq!(s.code_id_u64(), Some(6036));
    }
}
