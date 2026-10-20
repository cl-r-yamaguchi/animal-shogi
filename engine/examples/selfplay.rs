//! level 総当たりの強さ検証。使い方: cargo run --release --example selfplay -- [games_per_pair] [max_nodes]
use animal_shogi_engine::ai::{choose_move, Board, Outcome};
use animal_shogi_engine::{AiOptions, Side};
use std::collections::HashMap;

/// level a (a_is_sente なら先手) と b の1局。結果: 1=aの勝ち, -1=bの勝ち, 0=引き分け
fn play(a: u8, b: u8, a_sente: bool, seed: u32, max_nodes: u32) -> i32 {
    let mut board = Board::initial();
    let mut seen: HashMap<Board, u32> = HashMap::new();
    for ply in 0..300u32 {
        *seen.entry(board.clone()).or_default() += 1;
        if seen[&board] >= 3 {
            return 0;
        }
        if let Outcome::Win(w) = board.outcome() {
            return if (w == Side::Sente) == a_sente { 1 } else { -1 };
        }
        let level = if (board.turn() == Side::Sente) == a_sente { a } else { b };
        let opts = AiOptions { level, max_nodes, seed: seed.wrapping_mul(1000).wrapping_add(ply) };
        let r = choose_move(&board.to_state(), &opts).expect("合法手あり");
        board = board.play(&r.r#move);
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(50);
    let max_nodes: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    println!("games/pair={games} (先後半々), level3 max_nodes={max_nodes}");
    for (hi, lo) in [(2u8, 1u8), (3, 2), (3, 1)] {
        let (mut w, mut l, mut d) = (0, 0, 0);
        for g in 0..games {
            match play(hi, lo, g % 2 == 0, g, max_nodes) {
                1 => w += 1,
                -1 => l += 1,
                _ => d += 1,
            }
        }
        println!("level{hi} vs level{lo}: {hi}の{w}勝 {l}敗 {d}分");
    }
}
