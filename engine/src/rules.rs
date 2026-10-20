//! ゲームルール (docs/spec.md §1・§2・§4.2)。
use crate::types::*;

const LION: &[(i8, i8)] = &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
const GIRAFFE: &[(i8, i8)] = &[(-1, 0), (0, -1), (0, 1), (1, 0)];
const ELEPHANT: &[(i8, i8)] = &[(-1, -1), (-1, 1), (1, -1), (1, 1)];
const CHICK: &[(i8, i8)] = &[(-1, 0)];
const HEN: &[(i8, i8)] = &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, 0)];

fn deltas(kind: PieceKind) -> &'static [(i8, i8)] {
    match kind {
        PieceKind::Lion => LION,
        PieceKind::Giraffe => GIRAFFE,
        PieceKind::Elephant => ELEPHANT,
        PieceKind::Chick => CHICK,
        PieceKind::Hen => HEN,
    }
}

fn opponent(side: Side) -> Side {
    match side {
        Side::Sente => Side::Gote,
        Side::Gote => Side::Sente,
    }
}

/// `from` にいる `piece` が動ける行き先（盤内のみ、自駒の有無は考慮しない）。
fn destinations(piece: Piece, from: u8) -> Vec<u8> {
    let (r, c) = ((from / 3) as i8, (from % 3) as i8);
    // 先手の「前」は dRow = -1、後手は +1（表の dRow の符号を反転）
    let sign = if piece.owner == Side::Sente { 1 } else { -1 };
    let mut v = Vec::new();
    for &(dr, dc) in deltas(piece.kind) {
        let (nr, nc) = (r + dr * sign, c + dc);
        if (0..4).contains(&nr) && (0..3).contains(&nc) {
            v.push((nr * 3 + nc) as u8);
        }
    }
    v
}

/// 敵陣最奥段。
fn goal_row(side: Side) -> u8 {
    if side == Side::Sente {
        0
    } else {
        3
    }
}

pub fn initial_state() -> GameState {
    use PieceKind::*;
    let mut board: [Option<Piece>; 12] = [None; 12];
    let mut put = |sq: usize, kind, owner| board[sq] = Some(Piece { kind, owner });
    put(0, Giraffe, Side::Gote);
    put(1, Lion, Side::Gote);
    put(2, Elephant, Side::Gote);
    put(4, Chick, Side::Gote);
    put(7, Chick, Side::Sente);
    put(9, Elephant, Side::Sente);
    put(10, Lion, Side::Sente);
    put(11, Giraffe, Side::Sente);
    let mut state = GameState {
        board,
        hands: Hands { sente: Hand::default(), gote: Hand::default() },
        turn: Side::Sente,
        history: vec![],
    };
    state.history.push(position_key(&state));
    state
}

pub fn position_key(state: &GameState) -> String {
    let mut s = String::with_capacity(28);
    for p in &state.board {
        s.push(match p {
            None => '.',
            Some(p) => {
                let c = match p.kind {
                    PieceKind::Lion => 'l',
                    PieceKind::Giraffe => 'g',
                    PieceKind::Elephant => 'e',
                    PieceKind::Chick => 'c',
                    PieceKind::Hen => 'h',
                };
                if p.owner == Side::Sente {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            }
        });
    }
    for h in [&state.hands.sente, &state.hands.gote] {
        s.push_str(&format!("/{}{}{}", h.giraffe, h.elephant, h.chick));
    }
    s.push('/');
    s.push(if state.turn == Side::Sente { 's' } else { 'g' });
    s
}

fn hand_of(state: &GameState, side: Side) -> &Hand {
    if side == Side::Sente {
        &state.hands.sente
    } else {
        &state.hands.gote
    }
}

fn hand_of_mut(state: &mut GameState, side: Side) -> &mut Hand {
    if side == Side::Sente {
        &mut state.hands.sente
    } else {
        &mut state.hands.gote
    }
}

fn hand_count(h: &Hand, kind: HandKind) -> u8 {
    match kind {
        HandKind::Giraffe => h.giraffe,
        HandKind::Elephant => h.elephant,
        HandKind::Chick => h.chick,
    }
}

fn hand_count_mut(h: &mut Hand, kind: HandKind) -> &mut u8 {
    match kind {
        HandKind::Giraffe => &mut h.giraffe,
        HandKind::Elephant => &mut h.elephant,
        HandKind::Chick => &mut h.chick,
    }
}

/// 終局判定を行わない手生成（§2.3 の順）。
fn gen_moves(state: &GameState) -> Vec<Move> {
    let side = state.turn;
    let mut moves = Vec::new();
    for from in 0..12u8 {
        if let Some(p) = state.board[from as usize] {
            if p.owner != side {
                continue;
            }
            let mut tos: Vec<u8> = destinations(p, from)
                .into_iter()
                .filter(|&to| state.board[to as usize].map_or(true, |q| q.owner != side))
                .collect();
            tos.sort_unstable();
            moves.extend(tos.into_iter().map(|to| Move::Move { from, to }));
        }
    }
    let hand = hand_of(state, side);
    for kind in [HandKind::Giraffe, HandKind::Elephant, HandKind::Chick] {
        if hand_count(hand, kind) == 0 {
            continue;
        }
        for to in 0..12u8 {
            if state.board[to as usize].is_none() {
                moves.push(Move::Drop { kind, to });
            }
        }
    }
    moves
}

pub fn legal_moves(state: &GameState) -> Vec<Move> {
    if game_status(state) != GameStatus::Playing {
        return Vec::new();
    }
    gen_moves(state)
}

pub fn apply_move(state: &GameState, mv: &Move) -> Result<GameState, MoveError> {
    if game_status(state) != GameStatus::Playing {
        return Err(MoveError::GameOver);
    }
    let side = state.turn;
    let mut next = state.clone();
    match *mv {
        Move::Move { from, to } => {
            if from >= 12 || to >= 12 {
                return Err(MoveError::InvalidSquare);
            }
            let piece = state.board[from as usize].ok_or(MoveError::NoPiece)?;
            if piece.owner != side {
                return Err(MoveError::NotOwnPiece);
            }
            if !destinations(piece, from).contains(&to) {
                return Err(MoveError::IllegalMovement);
            }
            if let Some(t) = state.board[to as usize] {
                if t.owner == side {
                    return Err(MoveError::OccupiedByOwn);
                }
                // ニワトリはヒヨコに戻る。ライオンは持ち駒にならない。
                let captured = match t.kind {
                    PieceKind::Giraffe => Some(HandKind::Giraffe),
                    PieceKind::Elephant => Some(HandKind::Elephant),
                    PieceKind::Chick | PieceKind::Hen => Some(HandKind::Chick),
                    PieceKind::Lion => None,
                };
                if let Some(k) = captured {
                    *hand_count_mut(hand_of_mut(&mut next, side), k) += 1;
                }
            }
            let mut moved = piece;
            if piece.kind == PieceKind::Chick && to / 3 == goal_row(side) {
                moved.kind = PieceKind::Hen;
            }
            next.board[from as usize] = None;
            next.board[to as usize] = Some(moved);
        }
        Move::Drop { kind, to } => {
            if to >= 12 {
                return Err(MoveError::InvalidSquare);
            }
            if hand_count(hand_of(state, side), kind) == 0 {
                return Err(MoveError::NoPieceInHand);
            }
            if state.board[to as usize].is_some() {
                return Err(MoveError::SquareOccupied);
            }
            *hand_count_mut(hand_of_mut(&mut next, side), kind) -= 1;
            let pk = match kind {
                HandKind::Giraffe => PieceKind::Giraffe,
                HandKind::Elephant => PieceKind::Elephant,
                HandKind::Chick => PieceKind::Chick,
            };
            next.board[to as usize] = Some(Piece { kind: pk, owner: side });
        }
    }
    next.turn = opponent(side);
    let key = position_key(&next);
    next.history.push(key);
    Ok(next)
}

pub fn game_status(state: &GameState) -> GameStatus {
    // 1. キャッチ
    let lions: Vec<Piece> = state
        .board
        .iter()
        .flatten()
        .filter(|p| p.kind == PieceKind::Lion)
        .copied()
        .collect();
    if lions.len() == 1 {
        return GameStatus::Won { winner: lions[0].owner, reason: WinReason::CaptureLion };
    }
    // 2. トライ
    let s = state.turn;
    for sq in 0..12u8 {
        if let Some(p) = state.board[sq as usize] {
            if p.kind == PieceKind::Lion && p.owner == s && sq / 3 == goal_row(s) {
                return GameStatus::Won { winner: s, reason: WinReason::Try };
            }
        }
    }
    // 3. 詰み相当
    if gen_moves(state).is_empty() {
        return GameStatus::Won { winner: opponent(s), reason: WinReason::NoMoves };
    }
    // 4. 千日手
    if let Some(last) = state.history.last() {
        if state.history.iter().filter(|k| *k == last).count() >= 3 {
            return GameStatus::Draw { reason: DrawReason::Repetition };
        }
    }
    GameStatus::Playing
}
