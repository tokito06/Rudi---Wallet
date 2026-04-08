mod wallet;
mod storage;
mod bitcoin;
mod solana;
mod ethereum;
mod networks;
mod making_tx;
mod tea;

use crossterm::{
    event::{self, Event, KeyCode, EnableBracketedPaste, DisableBracketedPaste, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> anyhow::Result<()> {
    // Load wallet and derive addresses
    let password = "cooling cool";
    let loaded = storage::load_wallet(password)?;
    let w = wallet::Wallet::from_mnemonic(&loaded.mnemonic)?;
    let seed = w.seed_bytes()?;
    let bitcoin_address = bitcoin::derive_address(&seed)?;
    let solana_address = solana::derive_address(&seed)?;

    let mut app = tea::model::App::new(bitcoin_address, solana_address, seed.to_vec());

    // Fetch balances before entering the UI
    if let Ok(btc) = networks::btc::bitcoin_network::get_btc_balance(&app.bitcoin_address) {
        app.btc_balance = btc;
    }
    if let Ok(sol) = networks::solana_network::get_sol_balance(&app.solana_address) {
        app.solana_balance = sol;
    }

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // TEA event loop
    loop {
        terminal.draw(|frame| tea::view::draw(frame, &app))?;

        let msg = match event::read()? {
            Event::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('e'), KeyModifiers::CONTROL) => tea::update::Msg::Quit,
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => tea::update::Msg::GoToSend,
                (KeyCode::Char('r'), KeyModifiers::CONTROL) => tea::update::Msg::GoToReceive,
                (KeyCode::Char('n'), KeyModifiers::CONTROL) => tea::update::Msg::ToggleNetwork,
                (KeyCode::Enter, _) if matches!(app.screen, tea::model::Screen::Send) => tea::update::Msg::SendTokens,
                (KeyCode::Esc, _)       => tea::update::Msg::GoHome,
                (KeyCode::Backspace, _) => tea::update::Msg::Backspace,
                (KeyCode::Char(c), _)   => tea::update::Msg::InputChar(c),
                (KeyCode::Tab, _)       => tea::update::Msg::NextField,
                _ => continue,
            },
            Event::Paste(text) => tea::update::Msg::Paste(text),
            _ => continue,
        };

        tea::update::update(&mut app, msg);

        if app.needs_balance_refresh {
            app.needs_balance_refresh = false;
            if let Ok(btc) = networks::btc::bitcoin_network::get_btc_balance(&app.bitcoin_address) {
                app.btc_balance = btc;
            }
            if let Ok(sol) = networks::solana_network::get_sol_balance(&app.solana_address) {
                app.solana_balance = sol;
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableBracketedPaste)?;
    Ok(())
}
