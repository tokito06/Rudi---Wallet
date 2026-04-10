mod wallet;
mod storage;
mod bitcoin;
mod solana;
mod ethereum;
mod networks;
mod making_tx;
mod tea;

use crossterm::{
    event::{self, Event, KeyCode, EnableBracketedPaste, DisableBracketedPaste, KeyModifiers, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> anyhow::Result<()> {
    let password = "cooling cool";
    
    let wallet_data = if storage::wallet_exists() {
        println!("Loading existing wallet...");
        storage::load_wallet(password)?
    } else {
        println!("No wallet found. Creating a new one");
        
        let wallet = wallet::Wallet::generate()
            .map_err(|e| anyhow::anyhow!("Failed to generate wallet: {}", e))?;
        
        println!("\nYOUR SEED PHRASE:");
        println!("{}", wallet.mnemonic);
        println!("Type 'yes' to confirm you have saved the seed phrase:");
        let mut confirmation = String::new();
        std::io::stdin().read_line(&mut confirmation)?;
        
        if confirmation.trim().to_lowercase() != "yes" {
            anyhow::bail!("Wallet creation cancelled - seed phrase not confirmed");
        }
        
        storage::save_wallet(&wallet.mnemonic, password)?;
        println!("Wallet saved successfully!\n");
        
        storage::WalletData { 
            mnemonic: wallet.mnemonic 
        }
    };
    
    let w = wallet::Wallet::from_mnemonic(&wallet_data.mnemonic)?;
    let seed = w.seed_bytes()?;
    let bitcoin_address = bitcoin::derive_address(&seed)?;
    let solana_address = solana::derive_address(&seed)?;
    let ethereum_address = ethereum::derive_address(&seed)?;
    println!("Derived Addresses:");
    println!("Bitcoin: {}", bitcoin_address);
    println!("Solana: {}", solana_address);
    println!("Ethereum: {}", ethereum_address);

    let mut app = tea::model::App::new(bitcoin_address, solana_address, ethereum_address, seed.to_vec());

    if let Ok(btc) = networks::btc::bitcoin_network::get_btc_balance(&app.bitcoin_address) {
        app.btc_balance = btc;
    }
    if let Ok(sol) = networks::solana_network::get_sol_balance(&app.solana_address) {
        app.solana_balance = sol;
    }
    if let Ok(eth) = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(networks::ethereum_network::get_eth_balance(&app.ethereum_address)) 
    {
        app.eth_balance = eth;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| tea::view::draw(frame, &app))?;

        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }

        let msg = match event::read()? {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Char('t'), KeyModifiers::CONTROL) => tea::update::Msg::Quit,
                    (KeyCode::Char('s'), KeyModifiers::CONTROL) => tea::update::Msg::GoToSend,
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => tea::update::Msg::GoToReceive,
                    (KeyCode::Char('n'), KeyModifiers::CONTROL) => tea::update::Msg::ToggleNetwork,
                    (KeyCode::Enter, _) if matches!(app.screen, tea::model::Screen::Send) => tea::update::Msg::SendTokens,
                    (KeyCode::Esc, _)       => tea::update::Msg::GoHome,
                    (KeyCode::Backspace, _) => tea::update::Msg::Backspace,
                    (KeyCode::Char(c), _)   => tea::update::Msg::InputChar(c),
                    (KeyCode::Tab, _)       => tea::update::Msg::NextField,
                    _ => continue,
                }
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
            if let Ok(eth) = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(networks::ethereum_network::get_eth_balance(&app.ethereum_address)) 
            {
                app.eth_balance = eth;
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableBracketedPaste)?;
    Ok(())
}  