use serde::{Deserialize, Serialize};
use crate::making_tx::Network;

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub txid: String,
    pub network: Network,
    pub direction: Direction,
    pub status: Status,
    pub timestamp: String,
    pub amount: f64,
    pub address_from: String,
    pub address_to: String,
    pub fee: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Status {
    Pending,
    Success,
    Rejected,
}
