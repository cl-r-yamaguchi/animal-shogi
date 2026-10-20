import type {
  AiResult, EngineClient, GameState, GameStatus, HandKind, Move, MoveError, Piece, Result, Side,
} from "../types";

/**
 * 単独開発・テスト用のモックエンジン。
 * 小さなルール実装を TS で持つ（本物は Rust/WASM）。
 * CPU は決定的な手（即勝ち優先、なければ先頭の合法手）を返す。
 */

const DIRS: Record<string, [number, number][]> = {
  lion: [[-1, -1], [-1, 0], [-1, 1], [0, -1], [0, 1], [1, -1], [1, 0], [1, 1]],
  giraffe: [[-1, 0], [0, -1], [0, 1], [1, 0]],
  elephant: [[-1, -1], [-1, 1], [1, -1], [1, 1]],
  chick: [[-1, 0]],
  hen: [[-1, -1], [-1, 0], [-1, 1], [0, -1], [0, 1], [1, 0]],
};
const HAND_KINDS: HandKind[] = ["giraffe", "elephant", "chick"];
const other = (s: Side): Side => (s === "sente" ? "gote" : "sente");
const CH: Record<string, string> = { lion: "l", giraffe: "g", elephant: "e", chick: "c", hen: "h" };

function key(s: Pick<GameState, "board" | "hands" | "turn">): string {
  const b = s.board
    .map((p) => (p ? (p.owner === "sente" ? CH[p.kind].toUpperCase() : CH[p.kind]) : "."))
    .join("");
  const hs = (x: Record<HandKind, number>) => `${x.giraffe}${x.elephant}${x.chick}`;
  return `${b}/${hs(s.hands.sente)}/${hs(s.hands.gote)}/${s.turn === "sente" ? "s" : "g"}`;
}

export function mockInitialState(): GameState {
  const P = (kind: Piece["kind"], owner: Side): Piece => ({ kind, owner });
  const board: (Piece | null)[] = Array(12).fill(null);
  board[0] = P("giraffe", "gote"); board[1] = P("lion", "gote"); board[2] = P("elephant", "gote");
  board[4] = P("chick", "gote"); board[7] = P("chick", "sente");
  board[9] = P("elephant", "sente"); board[10] = P("lion", "sente"); board[11] = P("giraffe", "sente");
  const z = () => ({ giraffe: 0, elephant: 0, chick: 0 });
  const s: GameState = { board, hands: { sente: z(), gote: z() }, turn: "sente", history: [] };
  s.history = [key(s)];
  return s;
}

export function mockStatus(s: GameState): GameStatus {
  const lions = s.board.filter((p) => p?.kind === "lion");
  if (lions.length === 1) return { type: "won", winner: lions[0]!.owner, reason: "captureLion" };
  const goal = s.turn === "sente" ? 0 : 3;
  const idx = s.board.findIndex((p) => p?.kind === "lion" && p.owner === s.turn);
  if (idx >= 0 && Math.floor(idx / 3) === goal) return { type: "won", winner: s.turn, reason: "try" };
  const cur = s.history[s.history.length - 1];
  if (s.history.filter((k) => k === cur).length >= 3) return { type: "draw", reason: "repetition" };
  return { type: "playing" };
}

function sameMove(a: Move, b: Move): boolean {
  if (a.type === "move") return b.type === "move" && a.from === b.from && a.to === b.to;
  return b.type === "drop" && a.kind === b.kind && a.to === b.to;
}

export function mockLegalMoves(s: GameState): Move[] {
  if (mockStatus(s).type !== "playing") return [];
  const out: Move[] = [];
  const fwd = s.turn === "sente" ? 1 : -1; // dRow は先手基準。後手は反転
  for (let from = 0; from < 12; from++) {
    const p = s.board[from];
    if (!p || p.owner !== s.turn) continue;
    const tos: number[] = [];
    for (const [dr, dc] of DIRS[p.kind]) {
      const r = Math.floor(from / 3) + dr * fwd;
      const c = (from % 3) + dc;
      if (r < 0 || r > 3 || c < 0 || c > 2) continue;
      const to = r * 3 + c;
      if (s.board[to]?.owner === s.turn) continue;
      tos.push(to);
    }
    tos.sort((a, b) => a - b);
    for (const to of tos) out.push({ type: "move", from, to });
  }
  for (const kind of HAND_KINDS) {
    if (s.hands[s.turn][kind] === 0) continue;
    for (let to = 0; to < 12; to++) if (!s.board[to]) out.push({ type: "drop", kind, to });
  }
  return out;
}

export function mockApply(s: GameState, mv: Move): Result<GameState, MoveError> {
  if (mockStatus(s).type !== "playing") return { ok: false, error: "gameOver" };
  const legal = mockLegalMoves(s).some((m) => sameMove(m, mv));
  if (!legal) return { ok: false, error: mv.type === "drop" ? "noPieceInHand" : "illegalMovement" };
  const board = s.board.slice();
  const hands = { sente: { ...s.hands.sente }, gote: { ...s.hands.gote } };
  if (mv.type === "drop") {
    hands[s.turn][mv.kind]--;
    board[mv.to] = { kind: mv.kind, owner: s.turn };
  } else {
    const p = board[mv.from]!;
    const cap = board[mv.to];
    if (cap && cap.kind !== "lion") hands[s.turn][cap.kind === "hen" ? "chick" : (cap.kind as HandKind)]++;
    let kind = p.kind;
    if (kind === "chick" && Math.floor(mv.to / 3) === (s.turn === "sente" ? 0 : 3)) kind = "hen";
    board[mv.from] = null;
    board[mv.to] = { kind, owner: s.turn };
  }
  const next: GameState = { board, hands, turn: other(s.turn), history: s.history };
  next.history = [...s.history, key(next)];
  return { ok: true, value: next };
}

export function createMockEngine(): EngineClient {
  return {
    async initialState() { return mockInitialState(); },
    async legalMoves(s) { return mockLegalMoves(s); },
    async applyMove(s, m) { return mockApply(s, m); },
    async gameStatus(s) { return mockStatus(s); },
    async chooseMove(s): Promise<AiResult> {
      const moves = mockLegalMoves(s);
      if (moves.length === 0) throw new Error("noMoves");
      const win = moves.find((m) => {
        const r = mockApply(s, m);
        if (!r.ok) return false;
        const st = mockStatus(r.value);
        return st.type === "won" && st.winner === s.turn;
      });
      return { move: win ?? moves[0], score: 0, depth: 0, nodes: moves.length };
    },
    dispose() {},
  };
}
