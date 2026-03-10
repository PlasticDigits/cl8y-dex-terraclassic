use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BlockResponse {
    pub block: Block,
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
