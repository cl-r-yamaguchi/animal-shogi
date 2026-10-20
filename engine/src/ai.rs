//! AI (担当B)。シグネチャは docs/spec.md §4.3。
//!
//! rules.rs に依存しない独自の高速盤面 [`Board`] と指し手生成を持つ。
//! ルールは spec §1・§2.6 に従う。rules との照合は tests/ai_test.rs の
//! `#[ignore]` テストで行う。
use crate::types::*;

const WIN: i32 = 10000;
const DEFAULT_MAX_NODES: u32 = 1_000_000;
const MAX_DEPTH: u32 = 12;

// 駒コード: 0=空、1 + owner*5 + kind (kind: 0 lion,1 giraffe,2 elephant,3 chick,4 hen)
const LION: u8 = 0;
const CHICK: u8 = 3;
const HEN: u8 = 4;

/// 先手視点の方向 (dRow, dCol)。
const DIRS: [&[(i8, i8)]; 5] = [
    &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)],
    &[(-1, 0), (0, -1), (0, 1), (1, 0)],
    &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
    &[(-1, 0)],
    &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, 0)],
];

/// 内部の手。`from == DROP` のとき `kind` は持ち駒種 (0 giraffe,1 elephant,2 chick)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct IMove {
    from: u8,
    kind: u8,
    to: u8,
}
const DROP: u8 = 255;

/// 勝敗判定の結果。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Playing,
    Win(Side),
}

/// AI 内部の盤面表現。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Board {
    cells: [u8; 12],
    hands: [[u8; 3]; 2],
    turn: usize,
}

fn side_idx(s: Side) -> usize {
    if s == Side::Sente {
        0
    } else {
        1
    }
}
fn idx_side(i: usize) -> Side {
    if i == 0 {
        Side::Sente
    } else {
        Side::Gote
    }
}
fn kind_idx(k: PieceKind) -> u8 {
    match k {
        PieceKind::Lion => 0,
        PieceKind::Giraffe => 1,
        PieceKind::Elephant => 2,
        PieceKind::Chick => 3,
        PieceKind::Hen => 4,
    }
}
fn idx_kind(i: u8) -> PieceKind {
    [PieceKind::Lion, PieceKind::Giraffe, PieceKind::Elephant, PieceKind::Chick, PieceKind::Hen][i as usize]
}
fn goal_row(side: usize) -> u8 {
    if side == 0 {
        0
    } else {
        3
    }
}

impl Board {
    pub fn initial() -> Board {
        let mut b = Board { cells: [0; 12], hands: [[0; 3]; 2], turn: 0 };
        // 後手: キリン ライオン ゾウ / 先手: ゾウ ライオン キリン
        let put = |c: &mut [u8; 12], sq: usize, owner: usize, k: u8| c[sq] = 1 + owner as u8 * 5 + k;
        put(&mut b.cells, 0, 1, 1);
        put(&mut b.cells, 1, 1, 0);
        put(&mut b.cells, 2, 1, 2);
        put(&mut b.cells, 4, 1, 3);
        put(&mut b.cells, 7, 0, 3);
        put(&mut b.cells, 9, 0, 2);
        put(&mut b.cells, 10, 0, 0);
        put(&mut b.cells, 11, 0, 1);
        b
    }

    pub fn from_state(s: &GameState) -> Board {
        let mut cells = [0u8; 12];
        for (i, c) in s.board.iter().enumerate() {
            if let Some(p) = c {
                cells[i] = 1 + side_idx(p.owner) as u8 * 5 + kind_idx(p.kind);
            }
        }
        let h = |h: &Hand| [h.giraffe, h.elephant, h.chick];
        Board { cells, hands: [h(&s.hands.sente), h(&s.hands.gote)], turn: side_idx(s.turn) }
    }

    /// history は空の GameState に変換する（AI は history を読まない）。
    pub fn to_state(&self) -> GameState {
        let mut board = [None; 12];
        for i in 0..12 {
            let c = self.cells[i];
            if c != 0 {
                let o = (c - 1) / 5;
                board[i] = Some(Piece { kind: idx_kind((c - 1) % 5), owner: idx_side(o as usize) });
            }
        }
        let h = |h: &[u8; 3]| Hand { giraffe: h[0], elephant: h[1], chick: h[2] };
        GameState {
            board,
            hands: Hands { sente: h(&self.hands[0]), gote: h(&self.hands[1]) },
            turn: idx_side(self.turn),
            history: vec![],
        }
    }

    pub fn turn(&self) -> Side {
        idx_side(self.turn)
    }

    /// spec §1.4 の 1〜3（千日手は含まない）。
    pub fn outcome(&self) -> Outcome {
        let mut lions = [false; 2];
        for &c in &self.cells {
            if c != 0 && (c - 1) % 5 == LION {
                lions[((c - 1) / 5) as usize] = true;
            }
        }
        match (lions[0], lions[1]) {
            (true, false) => return Outcome::Win(Side::Sente),
            (false, true) => return Outcome::Win(Side::Gote),
            (false, false) => return Outcome::Win(idx_side(1 - self.turn)),
            _ => {}
        }
        if self.turn_lion_on_goal() {
            return Outcome::Win(idx_side(self.turn));
        }
        let mut v = Vec::new();
        self.gen(&mut v);
        if v.is_empty() {
            return Outcome::Win(idx_side(1 - self.turn));
        }
        Outcome::Playing
    }

    fn turn_lion_on_goal(&self) -> bool {
        let r = goal_row(self.turn) as usize;
        (0..3).any(|c| self.cells[r * 3 + c] == 1 + self.turn as u8 * 5 + LION)
    }

    /// 合法手を spec §2.3 の順で生成（終局判定はしない）。
    fn gen(&self, out: &mut Vec<IMove>) {
        let me = self.turn as u8;
        let mut tos: Vec<u8> = Vec::with_capacity(8);
        for from in 0..12u8 {
            let c = self.cells[from as usize];
            if c == 0 || (c - 1) / 5 != me {
                continue;
            }
            let k = (c - 1) % 5;
            let (r, col) = ((from / 3) as i8, (from % 3) as i8);
            tos.clear();
            for &(dr, dc) in DIRS[k as usize] {
                let dr = if me == 0 { dr } else { -dr };
                let (nr, nc) = (r + dr, col + dc);
                if !(0..4).contains(&nr) || !(0..3).contains(&nc) {
                    continue;
                }
                let to = (nr * 3 + nc) as u8;
                let t = self.cells[to as usize];
                if t != 0 && (t - 1) / 5 == me {
                    continue;
                }
                tos.push(to);
            }
            tos.sort_unstable();
            for &to in &tos {
                out.push(IMove { from, kind: 0, to });
            }
        }
        for kind in 0..3u8 {
            if self.hands[self.turn][kind as usize] == 0 {
                continue;
            }
            for to in 0..12u8 {
                if self.cells[to as usize] == 0 {
                    out.push(IMove { from: DROP, kind, to });
                }
            }
        }
    }

    fn captures_lion(&self, m: &IMove) -> bool {
        let t = self.cells[m.to as usize];
        m.from != DROP && t != 0 && (t - 1) % 5 == LION
    }

    fn make(&self, m: &IMove) -> Board {
        let mut b = self.clone();
        let me = self.turn as u8;
        if m.from == DROP {
            b.hands[self.turn][m.kind as usize] -= 1;
            b.cells[m.to as usize] = 1 + me * 5 + m.kind + 1;
        } else {
            let t = b.cells[m.to as usize];
            if t != 0 {
                let k = (t - 1) % 5;
                if k != LION {
                    let h = if k == HEN { 2 } else { k - 1 };
                    b.hands[self.turn][h as usize] += 1;
                }
            }
            let p = b.cells[m.from as usize];
            let mut k = (p - 1) % 5;
            if k == CHICK && m.to / 3 == goal_row(self.turn) {
                k = HEN;
            }
            b.cells[m.from as usize] = 0;
            b.cells[m.to as usize] = 1 + me * 5 + k;
        }
        b.turn = 1 - self.turn;
        b
    }

    /// 即勝ち: ライオン捕獲、または最奥段到達で相手に取り返されない。
    fn is_immediate_win(&self, m: &IMove) -> Option<i32> {
        if self.captures_lion(m) {
            return Some(WIN - 1);
        }
        let c = self.make(m);
        // 相手のライオンが最奥段にいれば、相手の手番で相手が先に勝つ
        if c.turn_lion_on_goal() {
            return None;
        }
        let mine = 1 + self.turn as u8 * 5 + LION;
        let reached = (0..3).any(|col| c.cells[goal_row(self.turn) as usize * 3 + col] == mine);
        if !reached {
            return None;
        }
        let mut v = Vec::new();
        c.gen(&mut v);
        if v.iter().any(|r| c.captures_lion(r)) {
            return None;
        }
        Some(WIN - 2)
    }

    fn eval(&self) -> i32 {
        const VAL: [i32; 5] = [0, 500, 500, 100, 300];
        let mut s = 0;
        for i in 0..12 {
            let c = self.cells[i];
            if c == 0 {
                continue;
            }
            let (o, k) = (((c - 1) / 5) as usize, (c - 1) % 5);
            let row = (i / 3) as i32;
            let adv = if o == 0 { 3 - row } else { row }; // 前進度 0..3
            let col = (i % 3) as i32;
            let mut v = VAL[k as usize];
            v += match k {
                LION => adv * 15,
                CHICK => adv * 20,
                HEN => 10,
                _ => (if col == 1 { 8 } else { 0 }) + adv * 3,
            };
            s += if o == self.turn { v } else { -v };
        }
        const HV: [i32; 3] = [550, 550, 130];
        for h in 0..3 {
            s += (self.hands[self.turn][h] as i32 - self.hands[1 - self.turn][h] as i32) * HV[h];
        }
        s
    }

    /// 合法手（終局なら空）。selfplay・テスト用。
    pub fn generate(&self) -> Vec<Move> {
        let mut v = Vec::new();
        if self.outcome() == Outcome::Playing {
            self.gen(&mut v);
        }
        v.iter().map(to_move).collect()
    }

    /// 手を適用する。不正な手・終局後は盤面を変えず Err を返す（panic しない）。
    pub fn play(&self, mv: &Move) -> Result<Board, MoveError> {
        if self.outcome() != Outcome::Playing {
            return Err(MoveError::GameOver);
        }
        let m = from_move(mv);
        let mut legal = Vec::new();
        self.gen(&mut legal);
        if legal.contains(&m) {
            return Ok(self.make(&m));
        }
        Err(match *mv {
            Move::Move { from, to } => {
                if from >= 12 || to >= 12 {
                    MoveError::InvalidSquare
                } else {
                    let c = self.cells[from as usize];
                    let t = self.cells[to as usize];
                    if c == 0 {
                        MoveError::NoPiece
                    } else if (c - 1) / 5 != self.turn as u8 {
                        MoveError::NotOwnPiece
                    } else if t != 0 && (t - 1) / 5 == self.turn as u8 {
                        MoveError::OccupiedByOwn
                    } else {
                        MoveError::IllegalMovement
                    }
                }
            }
            Move::Drop { to, .. } => {
                if to >= 12 {
                    MoveError::InvalidSquare
                } else if m.kind < 3 && self.hands[self.turn][m.kind as usize] == 0 {
                    MoveError::NoPieceInHand
                } else {
                    MoveError::SquareOccupied
                }
            }
        })
    }
}

fn to_move(m: &IMove) -> Move {
    if m.from == DROP {
        let kind = [HandKind::Giraffe, HandKind::Elephant, HandKind::Chick][m.kind as usize];
        Move::Drop { kind, to: m.to }
    } else {
        Move::Move { from: m.from, to: m.to }
    }
}

fn from_move(m: &Move) -> IMove {
    match *m {
        Move::Move { from, to } => IMove { from, kind: 0, to },
        Move::Drop { kind, to } => IMove {
            from: DROP,
            kind: match kind {
                HandKind::Giraffe => 0,
                HandKind::Elephant => 1,
                HandKind::Chick => 2,
            },
            to,
        },
    }
}

/// 状態から合法手を spec §2.3 の順で返す（rules との照合用）。終局なら空。
pub fn generate_moves(state: &GameState) -> Vec<Move> {
    Board::from_state(state).generate()
}

struct Rng(u64);
impl Rng {
    fn new(seed: u32) -> Rng {
        Rng((seed as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ 0xD1B54A32D192ED03)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

struct Search {
    nodes: u32,
    limit: u32, // 0 = 無制限
    aborted: bool,
}

impl Search {
    fn negamax(&mut self, b: &Board, depth: u32, mut alpha: i32, beta: i32, ply: i32) -> i32 {
        self.nodes += 1;
        if self.limit != 0 && self.nodes >= self.limit {
            self.aborted = true;
            return 0;
        }
        if b.turn_lion_on_goal() {
            return WIN - ply;
        }
        if depth == 0 {
            // 相手のライオンが最奥段にいる場合、こちらが取れなければ相手のトライ勝ち
            let opp = 1 + (1 - b.turn) as u8 * 5 + LION;
            let r = goal_row(1 - b.turn) as usize;
            if (0..3).any(|c| b.cells[r * 3 + c] == opp) {
                let mut v = Vec::with_capacity(32);
                b.gen(&mut v);
                return if v.iter().any(|m| b.captures_lion(m)) {
                    WIN - (ply + 1)
                } else {
                    -(WIN - (ply + 2))
                };
            }
            return b.eval();
        }
        let mut moves = Vec::with_capacity(32);
        b.gen(&mut moves);
        if moves.is_empty() {
            return -(WIN - ply);
        }
        order(b, &mut moves);
        let mut best = -WIN * 2;
        for m in &moves {
            let s = if b.captures_lion(m) {
                WIN - (ply + 1)
            } else {
                -self.negamax(&b.make(m), depth - 1, -beta, -alpha, ply + 1)
            };
            if self.aborted {
                return 0;
            }
            if s > best {
                best = s;
            }
            if s > alpha {
                alpha = s;
            }
            if alpha >= beta {
                break;
            }
        }
        best
    }
}

/// 取る手を優先（安定ソート）。価値順: ライオン > ゾウ/キリン/ニワトリ > ヒヨコ > 取らない手。

fn order(b: &Board, moves: &mut [IMove]) {
    moves.sort_by_key(|m| {
        let t = b.cells[m.to as usize];
        if m.from == DROP || t == 0 {
            0
        } else {
            match (t - 1) % 5 {
                LION => -100,
                CHICK => -10,
                _ => -50,
            }
        }
    });
}

/// AI の指し手を選ぶ。合法手がなければ None。
pub fn choose_move(state: &GameState, opts: &AiOptions) -> Option<AiResult> {
    let b = Board::from_state(state);
    if b.outcome() != Outcome::Playing {
        return None;
    }
    let mut moves = Vec::new();
    b.gen(&mut moves);
    let mut rng = Rng::new(opts.seed);
    let level = opts.level.clamp(1, 3);

    // 即勝ち（全 level 共通）。最も早い勝ちを優先。
    let mut win: Option<(i32, IMove)> = None;
    for m in &moves {
        if let Some(s) = b.is_immediate_win(m) {
            if win.map_or(true, |(bs, _)| s > bs) {
                win = Some((s, *m));
            }
        }
    }
    if let Some((score, m)) = win {
        return Some(AiResult {
            r#move: to_move(&m),
            score,
            depth: if level == 1 { 0 } else { 1 },
            nodes: moves.len() as u32,
        });
    }

    if level == 1 {
        let m = moves[rng.below(moves.len())];
        return Some(AiResult { r#move: to_move(&m), score: 0, depth: 0, nodes: moves.len() as u32 });
    }

    order(&b, &mut moves);
    if level == 2 {
        let mut s = Search { nodes: 0, limit: 0, aborted: false };
        let mut scored: Vec<(i32, IMove)> = Vec::new();
        for m in &moves {
            let sc = -s.negamax(&b.make(m), 2, -WIN * 2, WIN * 2, 1);
            scored.push((sc, *m));
        }
        let top = scored.iter().map(|x| x.0).max().unwrap();
        let best: Vec<&(i32, IMove)> = scored.iter().filter(|x| x.0 == top).collect();
        let m = best[rng.below(best.len())].1;
        return Some(AiResult { r#move: to_move(&m), score: top, depth: 3, nodes: s.nodes });
    }

    // level 3: 反復深化
    let limit = if opts.max_nodes == 0 { DEFAULT_MAX_NODES } else { opts.max_nodes };
    let mut s = Search { nodes: 0, limit: 0, aborted: false };
    let mut result: Option<(IMove, i32, u32)> = None;
    let mut prev_best: Option<IMove> = None;
    for d in 1..=MAX_DEPTH {
        s.limit = if d == 1 { 0 } else { limit }; // 深さ1は必ず完了させる
        if let Some(pm) = prev_best {
            if let Some(pos) = moves.iter().position(|m| *m == pm) {
                moves[..=pos].rotate_right(1); // 前回の最善手を先頭へ
            }
        }
        // 同点手を厳密に拾うため、下限は best-1（best と同値の手は厳密値で返る）
        let mut best_score = -WIN * 2;
        let mut ties: Vec<IMove> = Vec::new();
        for m in &moves {
            let lower = if ties.is_empty() { -WIN * 2 } else { best_score - 1 };
            let sc = -s.negamax(&b.make(m), d - 1, -WIN * 2, -lower, 1);
            if s.aborted {
                break;
            }
            if ties.is_empty() || sc > best_score {
                best_score = sc;
                ties.clear();
                ties.push(*m);
            } else if sc == best_score {
                ties.push(*m);
            }
        }
        if s.aborted {
            break;
        }
        let m = ties[rng.below(ties.len())];
        prev_best = Some(ties[0]);
        result = Some((m, best_score, d));
        if best_score.abs() > WIN - 100 {
            break; // 勝敗確定
        }
    }
    let (m, score, depth) = result.unwrap();
    Some(AiResult { r#move: to_move(&m), score, depth, nodes: s.nodes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(cells: &str, turn: usize) -> Board {
        let mut c = [0u8; 12];
        for (i, ch) in cells.chars().enumerate() {
            if ch == '.' {
                continue;
            }
            let k = "lgech".find(ch.to_ascii_lowercase()).unwrap() as u8;
            c[i] = 1 + if ch.is_ascii_uppercase() { 0 } else { 5 } + k;
        }
        Board { cells: c, hands: [[0; 3]; 2], turn }
    }

    #[test]
    fn order_prefers_valuable_captures() {
        // 先手キリン(4)が 後手ライオン(1)・ヒヨコ(3)・ゾウ(5) を取れる
        let b = board(".l.cGe.....L", 0);
        let mut v = Vec::new();
        b.gen(&mut v);
        order(&b, &mut v);
        let caps: Vec<u8> = v.iter().filter(|m| b.cells[m.to as usize] != 0).map(|m| m.to).collect();
        assert_eq!(caps, vec![1, 5, 3]); // ライオン > ゾウ > ヒヨコ
        assert_eq!(v[0].to, 1);
    }

    #[test]
    fn leaf_sees_unanswered_try() {
        // 後手ライオンが後手の最奥段(sq10)にいて、先手(手番)は取れない → 負け評価
        let b = board("...L......l.", 0);
        let mut s = Search { nodes: 0, limit: 0, aborted: false };
        assert!(s.negamax(&b, 0, -2 * WIN, 2 * WIN, 0) < -9000);
        // 先手が取れるなら勝ち評価
        let b = board("..........lG", 0);
        let mut s = Search { nodes: 0, limit: 0, aborted: false };
        assert!(s.negamax(&b, 0, -2 * WIN, 2 * WIN, 0) > 9000);
    }

    #[test]
    fn play_rejects_invalid_moves() {
        let b = Board::initial();
        for mv in [
            Move::Move { from: 0, to: 3 },  // 相手の駒
            Move::Move { from: 5, to: 4 },  // 空
            Move::Move { from: 12, to: 4 }, // 盤外
            Move::Move { from: 10, to: 7 }, // 自駒
            Move::Drop { kind: HandKind::Chick, to: 5 }, // 持ち駒なし
        ] {
            assert!(b.play(&mv).is_err(), "{mv:?}");
        }
        assert!(b.play(&Move::Move { from: 7, to: 4 }).is_ok());
    }
}
