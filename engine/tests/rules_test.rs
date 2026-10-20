use animal_shogi_engine::rules::*;
use animal_shogi_engine::*;

/// 盤12文字（positionKey と同じ表記）・先手持ち駒(g,e,c)・後手持ち駒・手番から局面を作る。
fn st(board: &str, sente: (u8, u8, u8), gote: (u8, u8, u8), turn: Side) -> GameState {
    let mut b: [Option<Piece>; 12] = [None; 12];
    for (i, ch) in board.chars().enumerate() {
        if ch == '.' {
            continue;
        }
        let kind = match ch.to_ascii_lowercase() {
            'l' => PieceKind::Lion,
            'g' => PieceKind::Giraffe,
            'e' => PieceKind::Elephant,
            'c' => PieceKind::Chick,
            'h' => PieceKind::Hen,
            _ => panic!("bad char"),
        };
        let owner = if ch.is_ascii_uppercase() { Side::Sente } else { Side::Gote };
        b[i] = Some(Piece { kind, owner });
    }
    let h = |t: (u8, u8, u8)| Hand { giraffe: t.0, elephant: t.1, chick: t.2 };
    let mut s = GameState {
        board: b,
        hands: Hands { sente: h(sente), gote: h(gote) },
        turn,
        history: vec![],
    };
    s.history.push(position_key(&s));
    s
}

fn mv(from: u8, to: u8) -> Move {
    Move::Move { from, to }
}

fn tos(state: &GameState, from: u8) -> Vec<u8> {
    legal_moves(state)
        .into_iter()
        .filter_map(|m| match m {
            Move::Move { from: f, to } if f == from => Some(to),
            _ => None,
        })
        .collect()
}

#[test]
fn initial_position() {
    let s = initial_state();
    assert_eq!(position_key(&s), "gle.c..C.ELG/000/000/s");
    assert_eq!(s.history, vec!["gle.c..C.ELG/000/000/s".to_string()]);
    assert_eq!(game_status(&s), GameStatus::Playing);
    assert_eq!(legal_moves(&s), vec![mv(7, 4), mv(10, 6), mv(10, 8), mv(11, 8)]);
}

#[test]
fn piece_movements_and_edges() {
    // 中央付近 (sq4) に各駒を置いた場合
    let s = st("l.........L.", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(tos(&s, 10), vec![6, 7, 8, 9, 11]);
    let s = st("l...G......L", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(tos(&s, 4), vec![1, 3, 5, 7]);
    let s = st("l...E......L", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(tos(&s, 4), vec![0, 2, 6, 8]);
    let s = st("l...C......L", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(tos(&s, 4), vec![1]);
    let s = st("l...H......L", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(tos(&s, 4), vec![0, 1, 2, 3, 5, 7]);
    // 後手は前後反転
    let s = st("l...c......L", (0, 0, 0), (0, 0, 0), Side::Gote);
    assert_eq!(tos(&s, 4), vec![7]);
    let s = st("l...h......L", (0, 0, 0), (0, 0, 0), Side::Gote);
    assert_eq!(tos(&s, 4), vec![1, 3, 5, 6, 7, 8]);
    // 盤端: 左上角のゾウ
    let s = st("E...l.....L.", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(tos(&s, 0), vec![4]);
    // 最奥段のヒヨコは動けない
    let s = st("C.l.......L.", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert!(tos(&s, 0).is_empty());
}

#[test]
fn promotion_on_move() {
    // 先手: 段1 -> 段0 に着くと成る
    let s = st("..l.C.....L.", (0, 0, 0), (0, 0, 0), Side::Sente);
    let n = apply_move(&s, &mv(4, 1)).unwrap();
    assert_eq!(n.board[1], Some(Piece { kind: PieceKind::Hen, owner: Side::Sente }));
    // 後手: 段2 -> 段3 に着くと成る
    let s = st("l......c...L", (0, 0, 0), (0, 0, 0), Side::Gote);
    let n = apply_move(&s, &mv(7, 10)).unwrap();
    assert_eq!(n.board[10], Some(Piece { kind: PieceKind::Hen, owner: Side::Gote }));
    // 途中の段では成らない
    let s = st("l.......C.L.", (0, 0, 0), (0, 0, 0), Side::Sente);
    let n = apply_move(&s, &mv(8, 5)).unwrap();
    assert_eq!(n.board[5].unwrap().kind, PieceKind::Chick);
}

#[test]
fn capture_to_hand_and_hen_reverts() {
    let s = st("l....h..G.L.", (0, 0, 0), (0, 0, 0), Side::Sente);
    // sente G at 8 moves up to 5 capturing hen
    let n = apply_move(&s, &mv(8, 5)).unwrap();
    assert_eq!(n.hands.sente, Hand { giraffe: 0, elephant: 0, chick: 1 });
    // ゾウ・キリン捕獲
    let s = st("l.e.g...EL..", (0, 0, 0), (0, 0, 0), Side::Sente);
    let n = apply_move(&s, &mv(8, 4)).unwrap();
    assert_eq!(n.hands.sente.giraffe, 1);
    assert_eq!(n.turn, Side::Gote);
    assert_eq!(n.history.len(), 2);
    assert_eq!(n.history.last().unwrap(), &position_key(&n));
}

#[test]
fn drop_moves() {
    let s = st("l..........L", (1, 0, 1), (0, 0, 0), Side::Sente);
    let moves = legal_moves(&s);
    // 持ち駒は giraffe → chick の順
    let drops: Vec<&Move> = moves.iter().filter(|m| matches!(m, Move::Drop { .. })).collect();
    assert_eq!(drops.len(), 20);
    assert_eq!(*drops[0], Move::Drop { kind: HandKind::Giraffe, to: 1 });
    assert_eq!(*drops[10], Move::Drop { kind: HandKind::Chick, to: 1 });
    // 最奥段に打っても成らない
    let n = apply_move(&s, &Move::Drop { kind: HandKind::Chick, to: 2 }).unwrap();
    assert_eq!(n.board[2], Some(Piece { kind: PieceKind::Chick, owner: Side::Sente }));
    assert_eq!(n.hands.sente.chick, 0);
    // 打てる駒種は持ち駒のみ（ゾウなし）
    assert!(!moves.iter().any(|m| matches!(m, Move::Drop { kind: HandKind::Elephant, .. })));
}

#[test]
fn catch_win() {
    let s = st("l.......E.L.", (0, 0, 0), (0, 0, 0), Side::Gote);
    // 盤上に先手ライオンがいる(sq10)ので継続
    assert_eq!(game_status(&s), GameStatus::Playing);
    let s = st("l.......E...", (0, 0, 0), (0, 0, 0), Side::Gote);
    assert_eq!(
        game_status(&s),
        GameStatus::Won { winner: Side::Gote, reason: WinReason::CaptureLion }
    );
    assert!(legal_moves(&s).is_empty());
    assert_eq!(apply_move(&s, &mv(8, 4)), Err(MoveError::GameOver));
    // 実際に取って終局
    let s = st(".l..L.......", (0, 0, 0), (0, 0, 0), Side::Sente);
    let n = apply_move(&s, &mv(4, 1)).unwrap();
    assert_eq!(n.hands.sente, Hand::default());
    assert_eq!(
        game_status(&n),
        GameStatus::Won { winner: Side::Sente, reason: WinReason::CaptureLion }
    );
}

#[test]
fn try_win_and_not_win() {
    // 先手ライオンが段0に到達した直後（後手番）は勝ちではない
    let s = st(".L.....l....", (0, 0, 0), (0, 0, 0), Side::Gote);
    assert_eq!(game_status(&s), GameStatus::Playing);
    // 取られずに手番が戻ってきた
    let s = st(".L.....l....", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(
        game_status(&s),
        GameStatus::Won { winner: Side::Sente, reason: WinReason::Try }
    );
    // 取られるケース: 後手のキリンが取れる
    let s = st("gL.....l....", (0, 0, 0), (0, 0, 0), Side::Gote);
    let n = apply_move(&s, &mv(0, 1)).unwrap();
    assert_eq!(
        game_status(&n),
        GameStatus::Won { winner: Side::Gote, reason: WinReason::CaptureLion }
    );
    // 後手のトライ
    let s = st("....L.......", (0, 0, 0), (0, 0, 0), Side::Gote);
    assert_eq!(game_status(&s), GameStatus::Won { winner: Side::Sente, reason: WinReason::CaptureLion });
    let s = st("....L..l....", (0, 0, 0), (0, 0, 0), Side::Gote);
    assert_eq!(game_status(&s), GameStatus::Playing);
    let s = st("....L.....l.", (0, 0, 0), (0, 0, 0), Side::Gote);
    assert_eq!(game_status(&s), GameStatus::Won { winner: Side::Gote, reason: WinReason::Try });
}

#[test]
fn no_moves_loses() {
    // 先手の全駒が動けない（ライオンは段3で目標段ではない）
    let s = st("CC.CC.CC.LCl", (0, 0, 0), (0, 0, 0), Side::Sente);
    assert_eq!(
        game_status(&s),
        GameStatus::Won { winner: Side::Gote, reason: WinReason::NoMoves }
    );
    assert!(legal_moves(&s).is_empty());
}

#[test]
fn repetition_draw() {
    let mut s = initial_state();
    // キリンを往復させる
    let seq = [mv(11, 8), mv(0, 3), mv(8, 11), mv(3, 0)];
    for round in 0..2 {
        for m in &seq {
            assert_eq!(game_status(&s), GameStatus::Playing, "round {round}");
            s = apply_move(&s, m).unwrap();
        }
    }
    // 初期局面が 3 回目
    assert_eq!(game_status(&s), GameStatus::Draw { reason: DrawReason::Repetition });
    assert!(legal_moves(&s).is_empty());
    assert_eq!(apply_move(&s, &seq[0]), Err(MoveError::GameOver));
}

#[test]
fn move_errors() {
    let s = initial_state();
    assert_eq!(apply_move(&s, &mv(12, 0)), Err(MoveError::InvalidSquare));
    assert_eq!(apply_move(&s, &mv(7, 99)), Err(MoveError::InvalidSquare));
    assert_eq!(apply_move(&s, &mv(3, 0)), Err(MoveError::NoPiece));
    assert_eq!(apply_move(&s, &mv(1, 0)), Err(MoveError::NotOwnPiece));
    assert_eq!(apply_move(&s, &mv(10, 0)), Err(MoveError::IllegalMovement));
    assert_eq!(apply_move(&s, &mv(10, 9)), Err(MoveError::OccupiedByOwn));
    assert_eq!(
        apply_move(&s, &Move::Drop { kind: HandKind::Chick, to: 3 }),
        Err(MoveError::NoPieceInHand)
    );
    assert_eq!(
        apply_move(&s, &Move::Drop { kind: HandKind::Chick, to: 12 }),
        Err(MoveError::InvalidSquare)
    );
    let s2 = st("l..........L", (0, 0, 1), (0, 0, 0), Side::Sente);
    assert_eq!(
        apply_move(&s2, &Move::Drop { kind: HandKind::Chick, to: 0 }),
        Err(MoveError::SquareOccupied)
    );
    // 優先順: 空マスかつ自駒への動き → NoPiece が先
    assert_eq!(apply_move(&s, &mv(3, 7)), Err(MoveError::NoPiece));
}

#[test]
fn random_playouts_consistent() {
    let mut seed: u64 = 12345;
    let mut next = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as usize
    };
    let mut finished = 0;
    for _ in 0..300 {
        let mut s = initial_state();
        for _ in 0..200 {
            let moves = legal_moves(&s);
            if game_status(&s) != GameStatus::Playing {
                assert!(moves.is_empty());
                finished += 1;
                break;
            }
            assert!(!moves.is_empty());
            let mut results = Vec::new();
            for m in &moves {
                let n = apply_move(&s, m).expect("legal move must apply");
                assert_eq!(n.history.last().unwrap(), &position_key(&n));
                assert_eq!(n.history.len(), s.history.len() + 1);
                results.push(n);
            }
            let idx = next() % moves.len();
            s = results.swap_remove(idx);
        }
    }
    assert!(finished > 0);
}

#[test]
fn json_shape() {
    let s = initial_state();
    let j = serde_json::to_value(&s).unwrap();
    assert_eq!(j["turn"], "sente");
    assert!(j["board"][3].is_null());
    assert_eq!(j["board"][1], serde_json::json!({"kind": "lion", "owner": "gote"}));
    let m = serde_json::to_value(Move::Drop { kind: HandKind::Chick, to: 3 }).unwrap();
    assert_eq!(m, serde_json::json!({"type": "drop", "kind": "chick", "to": 3}));
}

#[test]
fn wasm_wrappers() {
    use animal_shogi_engine::wasm::*;
    let init: serde_json::Value = serde_json::from_str(&initial_state_js()).unwrap();
    assert_eq!(init["ok"], true);
    let state_json = init["value"].to_string();
    let lm: serde_json::Value = serde_json::from_str(&legal_moves_js(&state_json)).unwrap();
    assert_eq!(lm["value"].as_array().unwrap().len(), 4);
    let r: serde_json::Value =
        serde_json::from_str(&apply_move_js(&state_json, r#"{"type":"move","from":1,"to":0}"#)).unwrap();
    assert_eq!(r, serde_json::json!({"ok": false, "error": "notOwnPiece"}));
    let r: serde_json::Value = serde_json::from_str(&legal_moves_js("not json")).unwrap();
    assert_eq!(r, serde_json::json!({"ok": false, "error": "invalidJson"}));
    let r: serde_json::Value = serde_json::from_str(&apply_move_js(&state_json, "{")).unwrap();
    assert_eq!(r["error"], "invalidJson");
    let r: serde_json::Value = serde_json::from_str(&game_status_js(&state_json)).unwrap();
    assert_eq!(r["value"]["type"], "playing");
}
