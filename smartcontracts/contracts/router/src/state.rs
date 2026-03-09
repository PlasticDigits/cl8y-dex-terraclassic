use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const FACTORY: Item<Addr> = Item::new("factory");
