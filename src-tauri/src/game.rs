use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GameType {
    Go,
    Gomoku,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Player {
    None,
    Black,
    White,
}

impl Player {
    pub fn other(&self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
            Player::None => Player::None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Game {
    pub board: Vec<Vec<Player>>,
    pub size: usize,
    pub current_turn: Player,
    pub last_move: Option<(usize, usize)>,
    // Simple Ko check: store hash of previous board states? 
    // For simplicity, just store the previous board state to check for simple Ko.
    pub previous_board: Option<Vec<Vec<Player>>>,
    pub game_type: GameType,
    pub winner: Option<Player>,
    pub is_draw: bool,
}

impl Game {
    pub fn new(size: usize, game_type: GameType) -> Self {
        let board = vec![vec![Player::None; size]; size];
        Game {
            board,
            size,
            current_turn: Player::Black,
            last_move: None,
            previous_board: None,
            game_type,
            winner: None,
            is_draw: false,
        }
    }

    pub fn play(&mut self, x: usize, y: usize) -> Result<bool, String> {
        if self.winner.is_some() || self.is_draw {
            return Err("Game is over".to_string());
        }
        if x >= self.size || y >= self.size {
            return Err("Out of bounds".to_string());
        }
        if self.board[y][x] != Player::None {
            return Err("Spot occupied".to_string());
        }

        if self.game_type == GameType::Gomoku {
            self.board[y][x] = self.current_turn;
            self.last_move = Some((x, y));
            
            if self.check_gomoku_win(x, y) {
                self.winner = Some(self.current_turn);
            } else {
                self.current_turn = self.current_turn.other();
            }
            return Ok(false);
        }

        let mut new_board = self.board.clone();
        new_board[y][x] = self.current_turn;

        // Check captures
        let opponent = self.current_turn.other();
        let mut captured = false;
        let mut stones_to_remove = HashSet::new();

        let neighbors = [(0, 1), (0, -1), (1, 0), (-1, 0)];

        for (dx, dy) in neighbors.iter() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && nx < self.size as i32 && ny >= 0 && ny < self.size as i32 {
                let nx = nx as usize;
                let ny = ny as usize;
                if new_board[ny][nx] == opponent {
                    if !self.has_liberties(&new_board, nx, ny) {
                        // Capture group
                        let group = self.get_group(&new_board, nx, ny);
                        for (gx, gy) in group {
                            stones_to_remove.insert((gx, gy));
                        }
                        captured = true;
                    }
                }
            }
        }

        for (rx, ry) in &stones_to_remove {
            new_board[*ry][*rx] = Player::None;
        }

        // Check suicide
        if !captured {
            if !self.has_liberties(&new_board, x, y) {
                return Err("Suicide move".to_string());
            }
        }

        // Check Ko
        if let Some(prev) = &self.previous_board {
            if new_board == *prev {
                return Err("Ko rule violation".to_string());
            }
        }

        self.previous_board = Some(self.board.clone());
        self.board = new_board;
        self.last_move = Some((x, y));
        self.current_turn = opponent;

        Ok(captured)
    }

    fn check_gomoku_win(&self, x: usize, y: usize) -> bool {
        let player = self.board[y][x];
        if player == Player::None { return false; }
        
        let directions = [(1, 0), (0, 1), (1, 1), (1, -1)];
        
        for (dx, dy) in directions.iter() {
            let mut count = 1;
            
            // Check forward
            let mut i = 1;
            loop {
                let nx = x as i32 + dx * i;
                let ny = y as i32 + dy * i;
                if nx < 0 || nx >= self.size as i32 || ny < 0 || ny >= self.size as i32 { break; }
                if self.board[ny as usize][nx as usize] == player {
                    count += 1;
                } else {
                    break;
                }
                i += 1;
            }
            
            // Check backward
            let mut i = 1;
            loop {
                let nx = x as i32 - dx * i;
                let ny = y as i32 - dy * i;
                if nx < 0 || nx >= self.size as i32 || ny < 0 || ny >= self.size as i32 { break; }
                if self.board[ny as usize][nx as usize] == player {
                    count += 1;
                } else {
                    break;
                }
                i += 1;
            }
            
            if count >= 5 {
                return true;
            }
        }
        false
    }

    fn has_liberties(&self, board: &Vec<Vec<Player>>, x: usize, y: usize) -> bool {
        let color = board[y][x];
        if color == Player::None {
            return true;
        }

        let mut visited = HashSet::new();
        let mut stack = vec![(x, y)];
        visited.insert((x, y));

        while let Some((cx, cy)) = stack.pop() {
            let neighbors = [(0, 1), (0, -1), (1, 0), (-1, 0)];
            for (dx, dy) in neighbors.iter() {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;

                if nx >= 0 && nx < self.size as i32 && ny >= 0 && ny < self.size as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    let neighbor_color = board[ny][nx];

                    if neighbor_color == Player::None {
                        return true;
                    }
                    if neighbor_color == color && !visited.contains(&(nx, ny)) {
                        visited.insert((nx, ny));
                        stack.push((nx, ny));
                    }
                }
            }
        }
        false
    }

    fn get_group(&self, board: &Vec<Vec<Player>>, x: usize, y: usize) -> Vec<(usize, usize)> {
        let color = board[y][x];
        let mut group = Vec::new();
        if color == Player::None {
            return group;
        }

        let mut visited = HashSet::new();
        let mut stack = vec![(x, y)];
        visited.insert((x, y));

        while let Some((cx, cy)) = stack.pop() {
            group.push((cx, cy));
            let neighbors = [(0, 1), (0, -1), (1, 0), (-1, 0)];
            for (dx, dy) in neighbors.iter() {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;

                if nx >= 0 && nx < self.size as i32 && ny >= 0 && ny < self.size as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    let neighbor_color = board[ny][nx];

                    if neighbor_color == color && !visited.contains(&(nx, ny)) {
                        visited.insert((nx, ny));
                        stack.push((nx, ny));
                    }
                }
            }
        }
        group
    }

    pub fn get_ai_move(&self) -> Option<(usize, usize)> {
        if self.game_type == GameType::Gomoku {
            return self.get_gomoku_ai_move();
        }

        let mut best_score = -1000;
        let mut best_moves = Vec::new();
        let size = self.size;
        
        // If board is empty, play 4-4 or 3-4
        let mut is_empty = true;
        'empty_check: for r in &self.board {
            for c in r {
                if *c != Player::None {
                    is_empty = false;
                    break 'empty_check;
                }
            }
        }
        if is_empty {
            return Some((3, 3)); // 4-4 point
        }

        for y in 0..size {
            for x in 0..size {
                if self.board[y][x] != Player::None {
                    continue;
                }

                let mut sim_game = self.clone();
                if let Ok(captured) = sim_game.play(x, y) {
                    let mut score = 0;
                    
                    // 1. Capture is good
                    if captured {
                        score += 100;
                    }

                    // 2. Avoid Self-Atari
                    let liberties = sim_game.get_liberty_count(x, y);
                    if liberties == 1 {
                        score -= 50; 
                    } else if liberties >= 3 {
                        score += 5;
                    }

                    // 3. Heuristics
                    // Prefer 3rd/4th line
                    if x == 2 || x == size - 3 || y == 2 || y == size - 3 { score += 2; }
                    if x == 3 || x == size - 4 || y == 3 || y == size - 4 { score += 3; }
                    
                    // Random noise
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    score += rng.gen_range(0..3);

                    if score > best_score {
                        best_score = score;
                        best_moves.clear();
                        best_moves.push((x, y));
                    } else if score == best_score {
                        best_moves.push((x, y));
                    }
                }
            }
        }

        if best_moves.is_empty() {
            return None;
        }
        
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..best_moves.len());
        Some(best_moves[idx])
    }

    fn get_gomoku_ai_move(&self) -> Option<(usize, usize)> {
        let mut best_score = -1;
        let mut best_moves = Vec::new();
        let size = self.size;
        let opponent = self.current_turn.other();

        // If board is empty, play center
        let center = size / 2;
        if self.board[center][center] == Player::None {
            return Some((center, center));
        }

        for y in 0..size {
            for x in 0..size {
                if self.board[y][x] != Player::None {
                    continue;
                }

                // Simple heuristic: Attack score + Defense score
                let attack_score = self.evaluate_gomoku_pos(x, y, self.current_turn);
                let defense_score = self.evaluate_gomoku_pos(x, y, opponent);
                
                // Weight defense slightly less than attack unless it's critical
                let score = attack_score + defense_score;

                if score > best_score {
                    best_score = score;
                    best_moves.clear();
                    best_moves.push((x, y));
                } else if score == best_score {
                    best_moves.push((x, y));
                }
            }
        }

        if best_moves.is_empty() {
            return None;
        }
        
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..best_moves.len());
        Some(best_moves[idx])
    }

    fn evaluate_gomoku_pos(&self, x: usize, y: usize, player: Player) -> i32 {
        let directions = [(1, 0), (0, 1), (1, 1), (1, -1)];
        let mut total_score = 0;

        for (dx, dy) in directions.iter() {
            let mut count = 1;
            let mut open_ends = 0;
            
            // Check forward
            let mut i = 1;
            loop {
                let nx = x as i32 + dx * i;
                let ny = y as i32 + dy * i;
                if nx < 0 || nx >= self.size as i32 || ny < 0 || ny >= self.size as i32 { break; }
                let cell = self.board[ny as usize][nx as usize];
                if cell == player {
                    count += 1;
                } else if cell == Player::None {
                    open_ends += 1;
                    break;
                } else {
                    break;
                }
                i += 1;
            }
            
            // Check backward
            let mut i = 1;
            loop {
                let nx = x as i32 - dx * i;
                let ny = y as i32 - dy * i;
                if nx < 0 || nx >= self.size as i32 || ny < 0 || ny >= self.size as i32 { break; }
                let cell = self.board[ny as usize][nx as usize];
                if cell == player {
                    count += 1;
                } else if cell == Player::None {
                    open_ends += 1;
                    break;
                } else {
                    break;
                }
                i += 1;
            }

            if count >= 5 {
                total_score += 100000;
            } else if count == 4 {
                if open_ends > 0 {
                    total_score += 10000; // Open 4 or Closed 4 (still dangerous)
                    if open_ends == 2 { total_score += 10000; } // Open 4 is winning
                }
            } else if count == 3 {
                if open_ends == 2 {
                    total_score += 1000; // Open 3
                } else if open_ends == 1 {
                    total_score += 100;
                }
            } else if count == 2 {
                if open_ends == 2 {
                    total_score += 100;
                }
            }
        }
        total_score
    }

    fn get_liberty_count(&self, x: usize, y: usize) -> usize {
        let group = self.get_group(&self.board, x, y);
        let mut liberties = HashSet::new();
        
        for (gx, gy) in group {
            let neighbors = [(0, 1), (0, -1), (1, 0), (-1, 0)];
            for (dx, dy) in neighbors.iter() {
                let nx = gx as i32 + dx;
                let ny = gy as i32 + dy;
                if nx >= 0 && nx < self.size as i32 && ny >= 0 && ny < self.size as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    if self.board[ny][nx] == Player::None {
                        liberties.insert((nx, ny));
                    }
                }
            }
        }
        liberties.len()
    }
}
