import { invoke } from '@tauri-apps/api/tauri';

async function init() {
    console.log('JS працює!');
    
    const app = document.getElementById('app');
    app.innerHTML = `
        <div style="text-align: center; padding: 50px;">
            <h1 style="color: #8b5cf6;">🦀 RuDi Wallet</h1>
            <p id="status">Завантаження...</p>
        </div>
    `;
    
    try {
        const exists = await invoke('wallet_exists');
        document.getElementById('status').innerHTML = exists ? '✅ Гаманець є' : '❌ Немає гаманця';
        document.getElementById('status').style.color = exists ? '#4ade80' : '#f87171';
    } catch (error) {
        document.getElementById('status').innerHTML = '❌ Помилка: ' + error;
    }
}

init();