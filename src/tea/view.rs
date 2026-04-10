use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::tea::model::{App, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.size());

    draw_header(frame, app, layout[0]);

    match app.screen {
        Screen::Home    => draw_home(frame, app, layout[1]),
        Screen::Send    => draw_send(frame, app, layout[1]),
        Screen::Receive => draw_receive(frame, app, layout[1]),
    }

    draw_footer(frame, app, layout[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = match app.screen {
        Screen::Home    => " RuDi Wallet — Home ",
        Screen::Send    => " RuDi Wallet — Send ",
        Screen::Receive => " RuDi Wallet — Receive ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    frame.render_widget(block, area);
}

fn draw_home(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // three equal columns now
    let cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    // Bitcoin panel
    let btc_text = vec![
        Line::from(Span::styled("Bitcoin (Testnet)", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::raw(format!("Address: {}", app.bitcoin_address))),
        Line::from(Span::raw(format!("Balance: {} BTC", app.btc_balance))),
    ];
    let btc = Paragraph::new(btc_text)
        .block(Block::default().borders(Borders::ALL).title(" BTC "));
    frame.render_widget(btc, cols[0]);

    // Solana panel
    let sol_text = vec![
        Line::from(Span::styled("Solana (Devnet)", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::raw(format!("Address: {}", app.solana_address))),
        Line::from(Span::raw(format!("Balance: {} SOL", app.solana_balance))),
    ];
    let sol = Paragraph::new(sol_text)
        .block(Block::default().borders(Borders::ALL).title(" SOL "));
    frame.render_widget(sol, cols[1]);

    // Ethereum panel
    let eth_text = vec![
        Line::from(Span::styled("Ethereum (Sepolia)", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::raw(format!("Address: {}", app.ethereum_address))),
        Line::from(Span::raw(format!("Balance: {} ETH", app.eth_balance))),
    ];
    let eth = Paragraph::new(eth_text)
        .block(Block::default().borders(Borders::ALL).title(" ETH "));
    frame.render_widget(eth, cols[2]);
}

fn draw_send(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let input = Paragraph::new(app.input_buffer.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Recipient Address "))
        .style(Style::default().fg(Color::White));
    frame.render_widget(input, rows[0]);

    let amount = Paragraph::new(app.amount_buffer.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Amount "))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(amount, rows[1]);

    // updated to show Eth as well
    let network_label = match app.network {
        crate::making_tx::Network::Btc => "BTC (Testnet)",
        crate::making_tx::Network::Sol => "SOL (Devnet)",
        crate::making_tx::Network::Eth => "ETH (Sepolia)",
    };
    let network_widget = Paragraph::new(network_label)
        .block(Block::default().borders(Borders::ALL).title(" Network  [Ctrl+N to toggle] "))
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(network_widget, rows[2]);

    let status = app.tx_result.as_deref().unwrap_or("Fill fields and press Enter to send.");
    let status_color = if app.tx_result.as_deref().unwrap_or("").starts_with("Error") {
        Color::Red
    } else {
        Color::Green
    };
    let hint = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(status_color));
    frame.render_widget(hint, rows[3]);
}

fn draw_receive(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let btc = Paragraph::new(app.bitcoin_address.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Your Bitcoin Address "))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(btc, rows[0]);

    let sol = Paragraph::new(app.solana_address.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Your Solana Address "))
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(sol, rows[1]);

    let eth = Paragraph::new(app.ethereum_address.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Your Ethereum Address "))
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(eth, rows[2]);
}

fn draw_footer(frame: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let keys = Paragraph::new(Line::from(vec![
        Span::styled(" [s] + Control ", Style::default().fg(Color::Green)),
        Span::raw("Send  "),
        Span::styled("[r] + Control ", Style::default().fg(Color::Green)),
        Span::raw("Receive  "),
        Span::styled("[Esc] ", Style::default().fg(Color::Green)),
        Span::raw("Home  "),
        Span::styled("[t] + Control ", Style::default().fg(Color::Red)),
        Span::raw("Quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(keys, area);
}