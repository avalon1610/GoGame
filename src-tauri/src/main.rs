#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::{State, Window};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod game;
use game::{Game, Player, GameType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
struct GameUpdate {
    board: Vec<Vec<Player>>,
    current_turn: Player,
    last_move: Option<(usize, usize)>,
    winner: Option<Player>,
    is_draw: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum NetworkMessage {
    Move(usize, usize),
    Resign,
    OfferDraw,
    AcceptDraw,
    RejectDraw,
    Restart(usize, GameType),
}

struct AppState {
    game: Mutex<Game>,
    // We use a channel to send moves to the network task if needed, 
    // but for simplicity, we might just write to a shared stream if we can lock it.
    // However, splitting the stream is better.
    // Let's just store if we are connected and let a background task handle incoming.
    // Outgoing moves can be sent via a channel or by cloning the stream (Arc<Mutex<TcpStream>>).
    tx: Mutex<Option<tokio::sync::mpsc::Sender<String>>>, 
}

#[tauri::command]
fn new_game(state: State<AppState>, size: usize, game_type: GameType) -> GameUpdate {
    let mut game = state.game.lock().unwrap();
    *game = Game::new(size, game_type);
    GameUpdate {
        board: game.board.clone(),
        current_turn: game.current_turn,
        last_move: game.last_move,
        winner: game.winner,
        is_draw: game.is_draw,
    }
}

#[tauri::command]
async fn play_move(
    state: State<'_, AppState>,
    x: usize,
    y: usize
) -> Result<GameUpdate, String> {
    let (update, sender) = {
        let mut game = state.game.lock().unwrap();
        
        // Apply move locally
        match game.play(x, y) {
            Ok(_) => {
                let update = GameUpdate {
                    board: game.board.clone(),
                    current_turn: game.current_turn,
                    last_move: game.last_move,
                    winner: game.winner,
                    is_draw: game.is_draw,
                };
                
                let tx_guard = state.tx.lock().unwrap();
                let sender = tx_guard.clone();
                
                (Ok(update), sender)
            }
            Err(e) => (Err(e), None),
        }
    };

    if let Some(s) = sender {
        let msg = serde_json::to_string(&NetworkMessage::Move(x, y)).unwrap();
        let _ = s.send(msg).await;
    }

    update
}

#[tauri::command]
async fn handle_game_action(
    state: State<'_, AppState>,
    action: String, // "resign", "offer_draw", "accept_draw", "reject_draw", "restart"
    payload: Option<String> // For restart: "size,type"
) -> Result<GameUpdate, String> {
    let (update, sender, msg_to_send) = {
        let mut game = state.game.lock().unwrap();
        let mut msg_to_send = None;

        match action.as_str() {
            "resign" => {
                game.winner = Some(game.current_turn.other());
                msg_to_send = Some(NetworkMessage::Resign);
            },
            "offer_draw" => {
                msg_to_send = Some(NetworkMessage::OfferDraw);
            },
            "accept_draw" => {
                game.is_draw = true;
                msg_to_send = Some(NetworkMessage::AcceptDraw);
            },
            "reject_draw" => {
                msg_to_send = Some(NetworkMessage::RejectDraw);
            },
            "restart" => {
                if let Some(p) = payload {
                    // payload format: "size,type" e.g. "19,Go"
                    let parts: Vec<&str> = p.split(',').collect();
                    if parts.len() == 2 {
                        let size = parts[0].parse().unwrap_or(19);
                        let gtype = match parts[1] {
                            "Gomoku" => GameType::Gomoku,
                            _ => GameType::Go,
                        };
                        *game = Game::new(size, gtype);
                        msg_to_send = Some(NetworkMessage::Restart(size, gtype));
                    }
                }
            },
            _ => {}
        }

        let update = GameUpdate {
            board: game.board.clone(),
            current_turn: game.current_turn,
            last_move: game.last_move,
            winner: game.winner,
            is_draw: game.is_draw,
        };
        
        let tx_guard = state.tx.lock().unwrap();
        let sender = tx_guard.clone();
        
        (Ok(update), sender, msg_to_send)
    };

    if let Some(s) = sender {
        if let Some(msg) = msg_to_send {
            let msg_str = serde_json::to_string(&msg).unwrap();
            let _ = s.send(msg_str).await;
        }
    }

    update
}

#[tauri::command]
async fn apply_remote_move(
    state: State<'_, AppState>,
    x: usize,
    y: usize
) -> Result<GameUpdate, String> {
    let mut game = state.game.lock().unwrap();
    
    match game.play(x, y) {
        Ok(_) => {
            Ok(GameUpdate {
                board: game.board.clone(),
                current_turn: game.current_turn,
                last_move: game.last_move,
                winner: game.winner,
                is_draw: game.is_draw,
            })
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn get_state(state: State<AppState>) -> GameUpdate {
    let game = state.game.lock().unwrap();
    GameUpdate {
        board: game.board.clone(),
        current_turn: game.current_turn,
        last_move: game.last_move,
        winner: game.winner,
        is_draw: game.is_draw,
    }
}

#[tauri::command]
async fn play_ai(state: State<'_, AppState>) -> Result<GameUpdate, String> {
    let mut game = state.game.lock().unwrap();
    
    if let Some((x, y)) = game.get_ai_move() {
        if game.play(x, y).is_ok() {
             return Ok(GameUpdate {
                board: game.board.clone(),
                current_turn: game.current_turn,
                last_move: game.last_move,
                winner: game.winner,
                is_draw: game.is_draw,
            });
        }
    }

    Err("AI could not find a move".to_string())
}

#[tauri::command]
async fn start_host(state: State<'_, AppState>, window: Window, port: u16) -> Result<String, String> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.map_err(|e| e.to_string())?;
    
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
    *state.tx.lock().unwrap() = Some(tx);

    tauri::async_runtime::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let (mut reader, mut writer) = socket.split();
            
            // Reader task
            let window_clone = window.clone();
            let mut buf = [0; 1024];
            
            loop {
                tokio::select! {
                    // Read from network
                    n = reader.read(&mut buf) => {
                        match n {
                            Ok(0) => break, // Connection closed
                            Ok(n) => {
                                let msg_str = String::from_utf8_lossy(&buf[0..n]);
                                // Try parsing as NetworkMessage
                                if let Ok(msg) = serde_json::from_str::<NetworkMessage>(&msg_str) {
                                    window_clone.emit("network-action", msg).unwrap();
                                } else if let Ok((x, y)) = serde_json::from_str::<(usize, usize)>(&msg_str) {
                                    // Backward compatibility or fallback
                                    window_clone.emit("network-action", NetworkMessage::Move(x, y)).unwrap();
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    // Write to network
                    Some(msg) = rx.recv() => {
                        let _ = writer.write_all(msg.as_bytes()).await;
                    }
                }
            }
        }
    });
    
    Ok("Host started".to_string())
}

#[tauri::command]
async fn connect_to_host(state: State<'_, AppState>, window: Window, ip: String) -> Result<String, String> {
    let socket = TcpStream::connect(ip).await.map_err(|e| e.to_string())?;
    
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
    *state.tx.lock().unwrap() = Some(tx);

    let (mut reader, mut writer) = socket.into_split();

    tauri::async_runtime::spawn(async move {
        let mut buf = [0; 1024];
        loop {
            tokio::select! {
                n = reader.read(&mut buf) => {
                    match n {
                        Ok(0) => break,
                        Ok(n) => {
                            let msg_str = String::from_utf8_lossy(&buf[0..n]);
                            if let Ok(msg) = serde_json::from_str::<NetworkMessage>(&msg_str) {
                                window.emit("network-action", msg).unwrap();
                            } else if let Ok((x, y)) = serde_json::from_str::<(usize, usize)>(&msg_str) {
                                window.emit("network-action", NetworkMessage::Move(x, y)).unwrap();
                            }
                        }
                        Err(_) => break,
                    }
                }
                Some(msg) = rx.recv() => {
                    let _ = writer.write_all(msg.as_bytes()).await;
                }
            }
        }
    });

    Ok("Connected".to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            game: Mutex::new(Game::new(19, GameType::Go)),
            tx: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            new_game,
            play_move,
            apply_remote_move,
            get_state,
            play_ai,
            start_host,
            connect_to_host,
            handle_game_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
