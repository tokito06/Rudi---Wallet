use crate::tea::model::{App, Screen, SendField};

pub enum Msg {
    GoToSend,
    GoToReceive,
    GoHome,
    InputChar(char),
    Backspace,
    Paste(String),
    BalanceLoaded(f64),
    NextField,
    ToggleNetwork,
    SendTokens,
    Quit,
}

pub fn update(app: &mut App, msg: Msg) {
    match msg {
        Msg::GoToSend         => app.screen = Screen::Send,
        Msg::GoToReceive      => app.screen = Screen::Receive,
        Msg::GoHome           => { app.screen = Screen::Home; 
                                   app.send_field = SendField::Address; 
                                   app.input_buffer.clear(); 
                                }
        Msg::InputChar(c)     => {
                                            if app.send_field == SendField::Address {
                                                app.input_buffer.push(c)
                                            }
                                            else {
                                                app.amount_buffer.push(c)
                                            }
                                        },
        Msg::Backspace        => { 
                                    if app.send_field == SendField::Address  {
                                        app.input_buffer.pop();
                                    }
                                    else {
                                        app.amount_buffer.pop(); 
                                    }
                                },
        Msg::BalanceLoaded(b) => app.solana_balance = b,
        Msg::Paste(s) => {
                                            if app.send_field == SendField::Address {
                                                app.input_buffer.push_str(&s)
                                            }
                                            else {
                                                app.amount_buffer.push_str(&s)
                                            }
                                        },
        Msg::ToggleNetwork    => {
            app.network = match app.network {
                crate::making_tx::Network::Btc => crate::making_tx::Network::Sol,
                crate::making_tx::Network::Sol => crate::making_tx::Network::Btc,
            };
        }
        Msg::SendTokens       => {
            if app.input_buffer.is_empty() || app.amount_buffer.is_empty() {
                app.tx_result = Some("Error: fields cannot be empty".to_string());
                return;
            }
            let amount: f64 = match app.amount_buffer.trim().parse() {
                Ok(v) => v,
                Err(_) => { 
                    app.tx_result = Some("Error: invalid amount".to_string()); return; }
            };
            let key = match app.network {
                crate::making_tx::Network::Btc => match crate::bitcoin::derive_private_key(&app.seed) {
                    Ok(k) => crate::making_tx::Key::Btc(k),
                    Err(e) => { app.tx_result = Some(format!("Error: {}", e)); return; }
                },
                crate::making_tx::Network::Sol => match crate::solana::derive_private_key(&app.seed) {
                    Ok(k) => crate::making_tx::Key::Sol(k.signing_key),
                    Err(e) => { app.tx_result = Some(format!("Error: {}", e)); return; }
                },
            };
            let sender = match app.network {
                crate::making_tx::Network::Btc => app.bitcoin_address.clone(),
                crate::making_tx::Network::Sol => app.solana_address.clone(),
            };
            let change = app.bitcoin_address.clone();
            match app.network.send(key, &sender, &app.input_buffer.clone(), amount, &change) {
                Ok(txid) => {
                    app.tx_result = Some(format!("Sent! TX: {}", txid));
                    app.needs_balance_refresh = true;
                }
                Err(e) => app.tx_result = Some(format!("Error: {}", e)),
            }
        }
        Msg::Quit             => app.should_quit = true,
        Msg::NextField        => app.send_field = SendField::Amount,
    }
}