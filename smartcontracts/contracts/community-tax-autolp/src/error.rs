use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Skim already in progress")]
    Reentrancy {},

    #[error("Router or pair is not configured")]
    RouterNotSet {},

    #[error("AutoLP LP policy is missing; manager must explicitly bind the initialized pair")]
    LpPolicyMissing {},

    #[error("AutoLP requires an initialized CW20 pool and a matching approved LP policy")]
    InvalidLpPool {},

    #[error("AutoLP pool reserves/share changed beyond the approved policy tolerance")]
    StaleLpPolicy {},

    #[error("AutoLP LP issuance below approved minimum")]
    InsufficientLp {},

    #[error("AutoLP swap returned no complete liquidity legs")]
    MissingLiquidityLeg {},

    /// Candidate is not a CL8Y factory pair that holds this tax token (M-3 / **M610-1**).
    #[error("Pair is not factory-listed or does not hold this tax token")]
    PairNotListed {},

    /// Manager tried to loosen skim `max_spread` past 200 bps (**M610-3**).
    #[error("Skim max_spread cannot exceed 200 bps")]
    SkimSpreadTooWide {},
}
