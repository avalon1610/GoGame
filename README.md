# Go Game (Rust + Tauri + React)

## Prerequisites
- Node.js
- Rust (cargo)

## Setup
1. Install dependencies:
   ```bash
   npm install
   ```

2. Run the app:
   ```bash
   npm run tauri dev
   ```

## Features
- **Play against AI**: Click "Play AI" to let the computer make a move.
- **Local Network Play**:
  - **Host**: Enter a port (e.g., 8080) and click "Host Game".
  - **Client**: Enter the Host's IP and Port (e.g., 192.168.1.5:8080) and click "Connect".
  - Moves are synchronized between Host and Client.

## Game Rules
- Simple Go rules (capture, suicide check, simple Ko).
- 19x19 board.
