export type Side = "sente" | "gote";
export type PieceKind = "lion" | "giraffe" | "elephant" | "chick" | "hen";
export type HandKind = "giraffe" | "elephant" | "chick";

export interface Piece {
  kind: PieceKind;
  owner: Side;
}

/** 0..=11。sq = row * 3 + col */
export type Square = number;

export type Hand = Record<HandKind, number>;

export interface GameState {
  board: (Piece | null)[];
  hands: Record<Side, Hand>;
  turn: Side;
  history: string[];
}

export type Move =
  | { type: "move"; from: Square; to: Square }
  | { type: "drop"; kind: HandKind; to: Square };

export type GameStatus =
  | { type: "playing" }
  | { type: "won"; winner: Side; reason: "captureLion" | "try" | "noMoves" }
  | { type: "draw"; reason: "repetition" };

export type MoveError =
  | "gameOver"
  | "invalidSquare"
  | "noPiece"
  | "notOwnPiece"
  | "illegalMovement"
  | "occupiedByOwn"
  | "noPieceInHand"
  | "squareOccupied";

export type Result<T, E = string> = { ok: true; value: T } | { ok: false; error: E };

export interface AiResult {
  move: Move;
  score: number;
  depth: number;
  nodes: number;
}

export interface AiOptions {
  maxNodes?: number;
  seed?: number;
}

export interface EngineClient {
  initialState(): Promise<GameState>;
  legalMoves(state: GameState): Promise<Move[]>;
  applyMove(state: GameState, move: Move): Promise<Result<GameState, MoveError>>;
  gameStatus(state: GameState): Promise<GameStatus>;
  chooseMove(state: GameState, level: 1 | 2 | 3, opts?: AiOptions): Promise<AiResult>;
  dispose(): void;
}
