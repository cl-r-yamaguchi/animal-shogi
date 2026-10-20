use animal_shogi_engine::ai::{choose_move, generate_moves, Board};
use animal_shogi_engine::*;

/// 盤面文字列(12文字, position_key と同じ表記: 大文字=先手)から局面を作る。history は空。
fn st(board: &str, sente: [u8; 3], gote: [u8; 3], turn: Side) -> GameState {
    assert_eq!(board.len(), 12);
    let mut b = [None; 12];
    for (i, ch) in board.chars().enumerate() {
        if ch == '.' {
            continue;
        }
        let kind = match ch.to_ascii_lowercase() {
            'l' => PieceKind::Lion,
            'g' => PieceKind::Giraffe,
            'e' => PieceKind::Elephant,
            'c' => PieceKind::Chick,
            _ => PieceKind::Hen,
        };
        let owner = if ch.is_ascii_uppercase() { Side::Sente } else { Side::Gote };
        b[i] = Some(Piece { kind, owner });
    }
    let h = |a: [u8; 3]| Hand { giraffe: a[0], elephant: a[1], chick: a[2] };
    GameState { board: b, hands: Hands { sente: h(sente), gote: h(gote) }, turn, history: vec![] }
}

fn opts(level: u8, seed: u32) -> AiOptions {
    AiOptions { level, max_nodes: 0, seed }
}

fn initial() -> GameState {
    Board::initial().to_state()
}

/// ランダム対局で収集した多様な局面
fn sample_states() -> Vec<GameState> {
    let mut out = vec![];
    for seed in 0..20usize {
        let mut b = Board::initial();
        for ply in 0..40usize {
            out.push(b.to_state());
            let mv = generate_moves(&b.to_state());
            if mv.is_empty() {
                break;
            }
            b = b.play(&mv[(seed * 7 + ply * 13) % mv.len()]).unwrap();
        }
    }
    out
}

#[test]
fn initial_moves_match_spec_vector() {
    assert_eq!(
        generate_moves(&initial()),
        vec![
            Move::Move { from: 7, to: 4 },
            Move::Move { from: 10, to: 6 },
            Move::Move { from: 10, to: 8 },
            Move::Move { from: 11, to: 8 },
        ]
    );
}

#[test]
fn all_levels_return_legal_moves() {
    for s in sample_states() {
        let legal = generate_moves(&s);
        for level in 1..=3u8 {
            let o = AiOptions { level, max_nodes: 20_000, seed: 3 };
            match choose_move(&s, &o) {
                Some(r) => assert!(legal.contains(&r.r#move), "level {level}"),
                None => assert!(legal.is_empty()),
            }
        }
    }
}

#[test]
fn finished_game_returns_none() {
    // ライオンが1体だけ（キャッチ済み）
    let s = st("..........L.", [0; 3], [0; 3], Side::Gote);
    for level in 1..=3 {
        assert!(choose_move(&s, &opts(level, 0)).is_none());
    }
}

#[test]
fn captures_lion_when_possible() {
    // 先手キリン(4)が後手ライオン(1)を取れる
    let s = st(".l..G......L", [0; 3], [0; 3], Side::Sente);
    for level in 1..=3 {
        for seed in 0..10 {
            let r = choose_move(&s, &opts(level, seed)).unwrap();
            assert_eq!(r.r#move, Move::Move { from: 4, to: 1 }, "level {level}");
            assert!(r.score > 9000);
        }
    }
}

#[test]
fn takes_try_win() {
    // 先手ライオン(3)が最奥段へ行ける。後手ライオン(8)は届かない。
    let s = st("...L....l...", [0; 3], [0; 3], Side::Sente);
    for level in 1..=3 {
        for seed in 0..10 {
            let r = choose_move(&s, &opts(level, seed)).unwrap();
            match r.r#move {
                Move::Move { from: 3, to } => assert!(to < 3, "level {level}"),
                m => panic!("level {level}: {m:?}"),
            }
        }
    }
}

#[test]
fn does_not_play_try_into_capture() {
    // 後手キリン(1)は sq2 を攻撃する。トライが成立するのは 5→1(キリンを取る) のみ。
    // 5→2 は取られるので、level1 でも選ばない。
    let s = st(".g...Ll.....", [0; 3], [0; 3], Side::Sente);
    for seed in 0..20 {
        let r = choose_move(&s, &opts(1, seed)).unwrap();
        assert_eq!(r.r#move, Move::Move { from: 5, to: 1 });
    }
}

#[test]
fn does_not_hang_lion() {
    // 後手ライオン(4)は先手キリン(7)に狙われている。7を取ると先手ライオン(11)に取り返されるので逃げるべき。
    let s = st("....l..G...L", [0; 3], [0; 3], Side::Gote);
    for level in 2..=3 {
        let r = choose_move(&s, &opts(level, 5)).unwrap();
        assert_ne!(r.r#move, Move::Move { from: 4, to: 7 }, "level {level}");
    }
}

#[test]
fn deterministic_for_same_input() {
    for s in sample_states().into_iter().step_by(7) {
        for level in 1..=3u8 {
            let o = AiOptions { level, max_nodes: 50_000, seed: 42 };
            assert_eq!(choose_move(&s, &o), choose_move(&s, &o));
        }
    }
}

#[test]
fn seed_changes_level1_choice() {
    let s = initial();
    let picks: std::collections::HashSet<String> = (0..50)
        .map(|seed| format!("{:?}", choose_move(&s, &opts(1, seed)).unwrap().r#move))
        .collect();
    assert!(picks.len() > 1);
}

#[test]
fn respects_max_nodes() {
    for s in sample_states().into_iter().step_by(11) {
        for limit in [100u32, 1_000, 10_000] {
            let o = AiOptions { level: 3, max_nodes: limit, seed: 1 };
            let Some(r) = choose_move(&s, &o) else { continue };
            // 超過は小幅（深さ1は必ず完了させる分を許容）
            assert!(r.nodes <= limit + 64, "nodes {} limit {}", r.nodes, limit);
        }
    }
}

#[test]
fn level3_default_max_nodes_when_zero() {
    let r = choose_move(&initial(), &opts(3, 0)).unwrap();
    assert!(r.nodes <= 1_000_000 + 64);
    assert!(r.depth >= 1);
}

/// rules.rs が実装された後に外す。理由: rules は担当Aが並行実装中で、AI ブランチでは todo!() のため。
#[test]
#[ignore = "rules.rs 実装後に有効化（担当Aの実装待ち）"]
fn matches_rules_legal_moves() {
    use animal_shogi_engine::rules;
    for seed in 0..100usize {
        let mut s = rules::initial_state();
        for ply in 0..60usize {
            let r = rules::legal_moves(&s);
            assert_eq!(r, generate_moves(&s), "seed {seed} ply {ply}");
            if r.is_empty() {
                break;
            }
            for level in 1..=3u8 {
                if let Some(a) = choose_move(&s, &AiOptions { level, max_nodes: 5000, seed: ply as u32 }) {
                    assert!(r.contains(&a.r#move));
                }
            }
            s = rules::apply_move(&s, &r[(seed * 31 + ply * 17) % r.len()]).unwrap();
            assert_eq!(Board::from_state(&s).to_state().board, s.board);
        }
    }
}

#[test]
fn level3_breaks_ties_by_seed() {
    // 左右対称な局面（列1にライオンのみ）では鏡像の手が同点になりうるため、seed で選択が変わる
    let boards = [".l...L.g....", ".l...L.G....", ".l.....L....", ".l..l..L....", ".l..L...L..."];
    let varies = boards.iter().any(|b| {
        let s = st(b, [0; 3], [0; 3], Side::Sente);
        let picks: std::collections::HashSet<String> = (0..40)
            .filter_map(|seed| choose_move(&s, &AiOptions { level: 3, max_nodes: 5_000, seed }))
            .map(|r| format!("{:?}", r.r#move))
            .collect();
        picks.len() > 1
    });
    assert!(varies);
}
