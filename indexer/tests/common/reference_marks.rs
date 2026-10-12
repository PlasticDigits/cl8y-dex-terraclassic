//! Complete original reference observations for owned request/producer fixtures.
//! Reference-source catalog rows without execution reserve views model still-fresh
//! independent reference publication while those source views are unavailable.
//! New source pools are explicitly execution-frozen after the original reference
//! publication, keeping them out of trading paths and provider work. This models
//! separate valuation/reference evidence and execution permission during setup.
use bigdecimal::BigDecimal;
use cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig;
use sqlx::PgPool;

pub async fn seed_reference_marks(
    pool: &PgPool,
    hub: &HubUsdConfig,
    overrides: &[(&str, BigDecimal)],
) {
    for address in [hub.custc_address.as_str(), hub.clunc_address.as_str()] {
        if !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE contract_address=$1)",
        )
        .bind(address)
        .fetch_one(pool)
        .await
        .unwrap()
        {
            sqlx::query("INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals) VALUES($1,true,'Canonical fixture wrap','untrusted-symbol',6)").bind(address).execute(pool).await.unwrap();
        }
    }
    let native: i32 = match sqlx::query_scalar("SELECT id FROM assets WHERE denom='uusd'")
        .fetch_optional(pool)
        .await
        .unwrap()
    {
        Some(id) => id,
        None => sqlx::query_scalar("INSERT INTO assets(denom,is_cw20,name,symbol,decimals) VALUES('uusd',false,'Native fixture anchor','untrusted-symbol',6) RETURNING id").fetch_one(pool).await.unwrap(),
    };
    for (ticker, address) in [
        ("ustc", hub.custc_address.as_str()),
        ("lunc", hub.clunc_address.as_str()),
    ] {
        sqlx::query("INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of)
            VALUES($1,1,'fresh',ARRAY['mexc','coingecko'],NOW()) ON CONFLICT(ticker)
            DO UPDATE SET quality='fresh',price_usd=1,accepted_sources=EXCLUDED.accepted_sources,as_of=NOW()")
            .bind(ticker).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO hub_prices(ticker,asset_id,price_usd,as_of,quality)
            SELECT $1,id,1,NOW(),'external_quorum' FROM assets WHERE contract_address=$2
            ON CONFLICT(ticker) DO UPDATE SET asset_id=EXCLUDED.asset_id,price_usd=1,as_of=NOW(),quality='external_quorum'")
            .bind(if ticker=="ustc" { "custc" } else { "lunc" }).bind(address)
            .execute(pool).await.unwrap();
    }
    let tokens: Vec<(i32, String, BigDecimal)> = sqlx::query_as(
        "SELECT a.id,a.contract_address,COALESCE(m.price_usd,1) FROM assets a LEFT JOIN economic_token_marks m ON m.contract_address=a.contract_address
        WHERE a.is_cw20 AND a.denom IS NULL
          AND a.contract_address ~ '^terra1[0-9a-z]{20,86}$'
          AND a.contract_address NOT IN ($1,$2)",
    )
    .bind(&hub.custc_address)
    .bind(&hub.clunc_address)
    .fetch_all(pool)
    .await
    .unwrap();
    for (id, address, existing_price) in tokens {
        let source:Option<(i32,String)> = sqlx::query_as("SELECT p.id,p.contract_address FROM pairs p
            JOIN assets a ON a.id=CASE WHEN p.asset_0_id=$1 THEN p.asset_1_id WHEN p.asset_1_id=$1 THEN p.asset_0_id END
            WHERE a.contract_address=$2 OR (NOT a.is_cw20 AND a.contract_address IS NULL AND a.denom='uusd')
            ORDER BY p.id LIMIT 1").bind(id).bind(&hub.custc_address).fetch_optional(pool).await.unwrap();
        let (source_id, source_address) = match source {
            Some(source) => source,
            None => {
                let reference_address = format!("terra1referencesource{id}1452");
                let source_id = sqlx::query_scalar(
                    "INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,fee_bps)
                    VALUES($1,$2,$3,30) RETURNING id",
                )
                .bind(&reference_address)
                .bind(id)
                .bind(native)
                .fetch_one(pool)
                .await
                .unwrap();
                (source_id, reference_address)
            }
        };
        if source_address.starts_with("terra1referencesource") {
            use cl8y_dex_indexer::indexer::asset_code_id_freeze::{
                apply_probe_verdicts, CodeIdFreezeVerdict,
            };
            apply_probe_verdicts(&[(source_address.clone(), CodeIdFreezeVerdict::Frozen)]);
        }
        sqlx::query("INSERT INTO economic_token_marks(contract_address,asset_id,price_usd,source_pair_id,source_pair_address,tvl_usd,as_of,anchor_ticker)
            VALUES($1,$2,$3,$4,$5,1000000,NOW(),'custc') ON CONFLICT(contract_address)
            DO UPDATE SET asset_id=EXCLUDED.asset_id,price_usd=EXCLUDED.price_usd,source_pair_id=EXCLUDED.source_pair_id,
                source_pair_address=EXCLUDED.source_pair_address,tvl_usd=EXCLUDED.tvl_usd,as_of=NOW(),anchor_ticker='custc'")
            .bind(&address).bind(id).bind(overrides.iter().find(|(contract,_)| *contract==address)
                .map(|(_,price)| price).unwrap_or(&existing_price))
            .bind(source_id).bind(source_address).execute(pool).await.unwrap();
    }
}
