//! WASM 公開関数 (docs/spec.md §4.4)。引数・戻り値は JSON 文字列。
use crate::types::*;
use crate::{ai, rules};
use serde::Serialize;
use wasm_bindgen::prelude::*;

fn ok<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(v) => format!("{{\"ok\":true,\"value\":{}}}", v),
        Err(_) => err("serializeFailed"),
    }
}

fn err<E: Serialize>(e: E) -> String {
    let e = serde_json::to_string(&e).unwrap_or_else(|_| "\"unknown\"".to_string());
    format!("{{\"ok\":false,\"error\":{}}}", e)
}

fn parse_state(json: &str) -> Result<GameState, String> {
    serde_json::from_str(json).map_err(|_| err("invalidJson"))
}

#[wasm_bindgen(js_name = initialState)]
pub fn initial_state_js() -> String {
    ok(&rules::initial_state())
}

#[wasm_bindgen(js_name = legalMoves)]
pub fn legal_moves_js(state_json: &str) -> String {
    match parse_state(state_json) {
        Ok(s) => ok(&rules::legal_moves(&s)),
        Err(e) => e,
    }
}

#[wasm_bindgen(js_name = applyMove)]
pub fn apply_move_js(state_json: &str, move_json: &str) -> String {
    let s = match parse_state(state_json) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mv: Move = match serde_json::from_str(move_json) {
        Ok(m) => m,
        Err(_) => return err("invalidJson"),
    };
    match rules::apply_move(&s, &mv) {
        Ok(n) => ok(&n),
        Err(e) => err(e),
    }
}

#[wasm_bindgen(js_name = gameStatus)]
pub fn game_status_js(state_json: &str) -> String {
    match parse_state(state_json) {
        Ok(s) => ok(&rules::game_status(&s)),
        Err(e) => e,
    }
}

#[wasm_bindgen(js_name = chooseMove)]
pub fn choose_move_js(state_json: &str, level: u8, max_nodes: u32, seed: u32) -> String {
    let s = match parse_state(state_json) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let opts = AiOptions { level, max_nodes, seed };
    match ai::choose_move(&s, &opts) {
        Some(r) => ok(&r),
        None => err("noMoves"),
    }
}
