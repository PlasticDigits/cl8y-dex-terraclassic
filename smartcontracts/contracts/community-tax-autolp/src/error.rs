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

    /// Candidate is not a CL8Y factory pair that holds this tax token (M-3 / **M610-1**).
    #[error("Pair is not factory-listed or does not hold this tax token")]
    PairNotListed {},

    /// Manager tried to loosen skim `max_spread` past 200 bps (**M610-3**).
    #[error("Skim max_spread cannot exceed 200 bps")]
    SkimSpreadTooWide {},
}
