pub enum Screen {
    Home,
    Send,
    Receive,
}
#[derive(PartialEq)]
pub enum  SendField{
    Address, 
    Amount,
}
pub struct App {
    pub screen: Screen,
    pub bitcoin_address: String,
    pub solana_address: String,
    pub ethereum_address: String,
    pub solana_balance: f64,
    pub btc_balance: f64,
    pub eth_balance: f64,
    pub input_buffer: String,
    pub should_quit: bool,
    pub send_field: SendField,
    pub amount_buffer: String,
    pub network: crate::making_tx::Network,
    pub tx_result: Option<String>,
    pub seed: Vec<u8>,
    pub needs_balance_refresh: bool,
}

impl App {
    pub fn new(bitcoin_address: String, solana_address: String, ethereum_address: String, seed: Vec<u8>) -> Self {
        App {
            screen: Screen::Home,
            bitcoin_address,
            solana_address,
            ethereum_address,
            solana_balance: 0.0,
            btc_balance: 0.0,
            eth_balance: 0.0,
            input_buffer: String::new(),
            should_quit: false,
            send_field: SendField::Address,
            amount_buffer: String::new(),
            network: crate::making_tx::Network::Btc,
            tx_result: None,
            seed,
            needs_balance_refresh: false,
        }
    }
}