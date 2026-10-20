//! 担当A が実装する。シグネチャは docs/spec.md §4.2。
use crate::types::*;

pub fn initial_state() -> GameState {
    todo!()
}
pub fn position_key(_state: &GameState) -> String {
    todo!()
}
pub fn legal_moves(_state: &GameState) -> Vec<Move> {
    todo!()
}
pub fn apply_move(_state: &GameState, _mv: &Move) -> Result<GameState, MoveError> {
    todo!()
}
pub fn game_status(_state: &GameState) -> GameStatus {
    todo!()
}
