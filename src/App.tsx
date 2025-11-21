import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import "./index.css";

type Player = "None" | "Black" | "White";
type GameType = "Go" | "Gomoku";

interface GameState {
  board: Player[][];
  current_turn: Player;
  last_move: [number, number] | null;
  winner: Player | null;
  is_draw: boolean;
}

type NetworkMessage = 
  | { Move: [number, number] }
  | "Resign"
  | "OfferDraw"
  | "AcceptDraw"
  | "RejectDraw"
  | { Restart: [number, GameType] };

function App() {
  const [gameState, setGameState] = useState<GameState | null>(null);
  const [status, setStatus] = useState("欢迎来到围棋/五子棋游戏");
  const [ip, setIp] = useState("127.0.0.1:8080");
  const [port, setPort] = useState("8080");
  const [isAiMode, setIsAiMode] = useState(false);
  const [gameType, setGameType] = useState<GameType>("Go");
  const [drawOfferedByOpponent, setDrawOfferedByOpponent] = useState(false);

  const playSound = (type: "move" | "win" | "lose" | "draw" = "move") => {
    try {
      const AudioContext = window.AudioContext || (window as any).webkitAudioContext;
      if (!AudioContext) return;
      
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();

      osc.connect(gain);
      gain.connect(ctx.destination);

      if (type === "move") {
        osc.type = "sine";
        osc.frequency.setValueAtTime(400, ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(100, ctx.currentTime + 0.1);
        gain.gain.setValueAtTime(0.3, ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + 0.1);
        osc.start();
        osc.stop(ctx.currentTime + 0.1);
      } else if (type === "win") {
        osc.type = "triangle";
        osc.frequency.setValueAtTime(300, ctx.currentTime);
        osc.frequency.linearRampToValueAtTime(600, ctx.currentTime + 0.2);
        osc.frequency.linearRampToValueAtTime(400, ctx.currentTime + 0.4);
        osc.frequency.linearRampToValueAtTime(800, ctx.currentTime + 0.6);
        gain.gain.setValueAtTime(0.3, ctx.currentTime);
        gain.gain.linearRampToValueAtTime(0, ctx.currentTime + 1.0);
        osc.start();
        osc.stop(ctx.currentTime + 1.0);
      } else if (type === "lose") {
        osc.type = "sawtooth";
        osc.frequency.setValueAtTime(200, ctx.currentTime);
        osc.frequency.linearRampToValueAtTime(100, ctx.currentTime + 0.5);
        gain.gain.setValueAtTime(0.3, ctx.currentTime);
        gain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.5);
        osc.start();
        osc.stop(ctx.currentTime + 0.5);
      } else if (type === "draw") {
        osc.type = "sine";
        osc.frequency.setValueAtTime(400, ctx.currentTime);
        osc.frequency.setValueAtTime(400, ctx.currentTime + 0.2);
        gain.gain.setValueAtTime(0.3, ctx.currentTime);
        gain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.5);
        osc.start();
        osc.stop(ctx.currentTime + 0.5);
      }
    } catch (e) {
      console.error("Audio error", e);
    }
  };

  const startNewGame = (type: GameType) => {
    setGameType(type);
    const size = type === "Go" ? 19 : 15;
    // If connected, we should send restart command
    invoke<GameState>("handle_game_action", { action: "restart", payload: `${size},${type}` }).then((state) => {
        setGameState(state);
        setStatus("游戏开始");
        setDrawOfferedByOpponent(false);
    }).catch(e => setStatus(`错误: ${e}`));
  };

  useEffect(() => {
    startNewGame("Go");

    const unlisten = listen<NetworkMessage>("network-action", (event: any) => {
      const msg = event.payload;
      console.log("Received network message:", msg);

      if (typeof msg === 'object' && 'Move' in msg) {
          const [x, y] = msg.Move;
          invoke<GameState>("apply_remote_move", { x, y })
            .then((state) => {
                setGameState(state);
                playSound("move");
                checkGameOver(state);
            });
      } else if (msg === "Resign") {
          invoke<GameState>("get_state").then(state => {
              setGameState(state);
              checkGameOver(state);
          });
      } else if (msg === "OfferDraw") {
          setDrawOfferedByOpponent(true);
      } else if (msg === "AcceptDraw") {
          invoke<GameState>("handle_game_action", { action: "accept_draw", payload: null }).then(state => {
              setGameState(state);
              checkGameOver(state);
          });
      } else if (msg === "RejectDraw") {
          setStatus("对方拒绝了求和");
      } else if (typeof msg === 'object' && 'Restart' in msg) {
          const [size, type] = msg.Restart;
          setGameType(type);
          invoke<GameState>("get_state").then(state => {
              setGameState(state);
              setStatus("游戏重新开始");
              setDrawOfferedByOpponent(false);
          });
      }
    });

    return () => {
      unlisten.then((f: any) => f());
    };
  }, []);

  const checkGameOver = (state: GameState) => {
      if (state.winner) {
          setStatus(`游戏结束! ${state.winner === "Black" ? "黑方" : "白方"} 获胜!`);
          playSound("win");
      } else if (state.is_draw) {
          setStatus("游戏结束! 平局!");
          playSound("draw");
      }
  };

  const handleCellClick = async (x: number, y: number) => {
    if (!gameState) return;
    if (gameState.winner || gameState.is_draw) return;

    try {
      const newState = await invoke<GameState>("play_move", { x, y });
      setGameState(newState);
      setStatus("");
      playSound("move");

      checkGameOver(newState);

      if (isAiMode && !newState.winner && !newState.is_draw) {
        setTimeout(handleAI, 200);
      }
    } catch (e) {
      setStatus(`错误: ${e}`);
    }
  };

  const handleAI = async () => {
    try {
      const newState = await invoke<GameState>("play_ai");
      setGameState(newState);
      setStatus("AI 已落子");
      playSound("move");
      checkGameOver(newState);
    } catch (e) {
      setStatus(`错误: ${e}`);
    }
  };

  const handleResign = async () => {
      if (!gameState || gameState.winner || gameState.is_draw) return;
      if (confirm("确定要认输吗?")) {
          const newState = await invoke<GameState>("handle_game_action", { action: "resign", payload: null });
          setGameState(newState);
          checkGameOver(newState);
      }
  };

  const handleOfferDraw = async () => {
      if (!gameState || gameState.winner || gameState.is_draw) return;
      await invoke("handle_game_action", { action: "offer_draw", payload: null });
      setStatus("已发送求和请求...");
  };

  const handleAcceptDraw = async () => {
      const newState = await invoke<GameState>("handle_game_action", { action: "accept_draw", payload: null });
      setGameState(newState);
      setDrawOfferedByOpponent(false);
      checkGameOver(newState);
  };

  const handleRejectDraw = async () => {
      await invoke("handle_game_action", { action: "reject_draw", payload: null });
      setDrawOfferedByOpponent(false);
  };

  const startHost = async () => {
    try {
      const res = await invoke<string>("start_host", { port: parseInt(port) });
      setStatus(res);
    } catch (e) {
      setStatus(`错误: ${e}`);
    }
  };

  const connectHost = async () => {
    try {
      const res = await invoke<string>("connect_to_host", { ip });
      setStatus(res);
    } catch (e) {
      setStatus(`错误: ${e}`);
    }
  };

  if (!gameState) return <div className="loading">加载中...</div>;

  return (
    <div className="container">
      {/* Draw Offer Modal */}
      {drawOfferedByOpponent && (
          <div className="modal-overlay">
              <div className="modal">
                  <h3>对方请求和棋</h3>
                  <div className="modal-buttons">
                      <button onClick={handleAcceptDraw}>接受</button>
                      <button onClick={handleRejectDraw}>拒绝</button>
                  </div>
              </div>
          </div>
      )}

      {/* Game Over Overlay */}
      {(gameState.winner || gameState.is_draw) && (
          <div className="game-over-overlay">
              <div className={`game-over-content ${gameState.winner ? 'win' : 'draw'}`}>
                  {gameState.winner ? (
                      <>
                        <h1>{gameState.winner === "Black" ? "黑方" : "白方"} 获胜!</h1>
                        <div className="trophy">🏆</div>
                      </>
                  ) : (
                      <h1>平局!</h1>
                  )}
                  <button onClick={() => startNewGame(gameType)}>重新开始</button>
              </div>
          </div>
      )}

      <div className="sidebar">
        <h1>Go / Gomoku</h1>
        <div className="controls">
            <div className="game-mode">
                <button className={gameType === "Go" ? "active" : ""} onClick={() => startNewGame("Go")}>围棋 (19x19)</button>
                <button className={gameType === "Gomoku" ? "active" : ""} onClick={() => startNewGame("Gomoku")}>五子棋 (15x15)</button>
            </div>
            
            <div className="action-buttons">
                <button onClick={() => startNewGame(gameType)} className="restart-btn">重新开始</button>
                <button onClick={handleResign} className="resign-btn">认输</button>
                <button onClick={handleOfferDraw} className="draw-btn">求和</button>
            </div>

            <div className="status-box">
                <p>{status}</p>
                <p>当前回合: {gameState.current_turn === "Black" ? "黑方" : "白方"}</p>
            </div>

            <div className="network-controls">
                <h3>网络对战</h3>
                <input value={port} onChange={e => setPort(e.target.value)} placeholder="端口" />
                <button onClick={startHost}>作为主机启动</button>
                <div className="divider"></div>
                <input value={ip} onChange={e => setIp(e.target.value)} placeholder="IP地址:端口" />
                <button onClick={connectHost}>连接主机</button>
            </div>

            <div className="ai-controls">
                <h3>单人模式</h3>
                <div className="ai-options">
                    <label className="checkbox-label">
                        <input type="checkbox" checked={isAiMode} onChange={e => setIsAiMode(e.target.checked)} />
                        启用 AI 对手
                    </label>
                    {isAiMode && <button onClick={handleAI} className="ai-act-btn">AI 立即行动</button>}
                </div>
            </div>
        </div>
      </div>

      <div className="game-area">
        <div className="board" style={{ 
            gridTemplateColumns: `repeat(${gameState.board.length}, 1fr)`,
            width: 'min(80vh, 80vw)',
            height: 'min(80vh, 80vw)'
        }}>
          {gameState.board.map((row, y) =>
            row.map((cell, x) => {
                const isLastMove = gameState.last_move && gameState.last_move[0] === x && gameState.last_move[1] === y;
                return (
                  <div
                    key={`${x}-${y}`}
                    className={`cell ${x === 0 ? 'left' : ''} ${x === gameState.board.length - 1 ? 'right' : ''} ${y === 0 ? 'top' : ''} ${y === gameState.board.length - 1 ? 'bottom' : ''}`}
                    onClick={() => handleCellClick(x, y)}
                  >
                    <div className="grid-line horizontal"></div>
                    <div className="grid-line vertical"></div>
                    {cell !== "None" && (
                      <div className={`stone ${cell.toLowerCase()} ${isLastMove ? 'last-move' : ''}`}>
                          {isLastMove && <div className="marker"></div>}
                      </div>
                    )}
                    {/* Star points (Hoshi) */}
                    {isStarPoint(x, y, gameState.board.length) && <div className="star-point"></div>}
                  </div>
                );
            })
          )}
        </div>
      </div>
    </div>
  );
}

function isStarPoint(x: number, y: number, size: number) {
    if (size === 19) {
        const points = [3, 9, 15];
        return points.includes(x) && points.includes(y);
    }
    if (size === 15) {
        const points = [3, 7, 11];
        return points.includes(x) && points.includes(y);
    }
    return false;
}

export default App;
