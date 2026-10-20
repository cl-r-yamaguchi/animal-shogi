use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Sente,
    Gote,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PieceKind {
    Lion,
    Giraffe,
    Elephant,
    Chick,
    Hen,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandKind {
    Giraffe,
    Elephant,
    Chick,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Piece {
    pub kind: PieceKind,
    pub owner: Side,
}

pub type Square = u8; // 0..=11

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Hand {
    pub giraffe: u8,
    pub elephant: u8,
    pub chick: u8,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Hands {
    pub sente: Hand,
    pub gote: Hand,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub board: [Option<Piece>; 12],
    pub hands: Hands,
    pub turn: Side,
    pub history: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Move {
    Move { from: Square, to: Square },
    Drop { kind: HandKind, to: Square },
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GameStatus {
    Playing,
    Won { winner: Side, reason: WinReason },
    Draw { reason: DrawReason },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WinReason {
    CaptureLion,
    Try,
    NoMoves,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawReason {
    Repetition,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveError {
    GameOver,
    InvalidSquare,
    NoPiece,
    NotOwnPiece,
    IllegalMovement,
    OccupiedByOwn,
    NoPieceInHand,
    SquareOccupied,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AiResult {
    pub r#move: Move,
    pub score: i32,
    pub depth: u32,
    pub nodes: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct AiOptions {
    pub level: u8,
    pub max_nodes: u32,
    pub seed: u32,
}
