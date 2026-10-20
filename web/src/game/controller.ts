import type {
  EngineClient, GameState, GameStatus, HandKind, Move, Side, Square,
} from "../types";

export type Level = 1 | 2 | 3;

export interface NewGameOptions {
  humanSide: Side;
  level: Level;
}

export type Selection =
  | { type: "none" }
  | { type: "square"; square: Square }
  | { type: "hand"; kind: HandKind };

export interface GameView {
  state: GameState;
  status: GameStatus;
  humanSide: Side;
  level: Level;
  selection: Selection;
  targets: Square[];
  lastMove: Move | null;
  thinking: boolean;
  canUndo: boolean;
  /** エンジン呼び出しの失敗メッセージ。なければ null */
  error: string | null;
}

export interface GameController {
  getView(): GameView;
  subscribe(listener: (view: GameView) => void): () => void;
  newGame(options: NewGameOptions): Promise<void>;
  clickSquare(square: Square): Promise<void>;
  clickHand(kind: HandKind): void;
  clearSelection(): void;
  undo(): void;
  /** エラー後に CPU の手番をやり直す */
  retry(): Promise<void>;
}

export interface ControllerConfig {
  /** CPU の手を反映するまでの最低待ち時間(ms)。既定 400。テストでは 0 を指定 */
  cpuDelayMs?: number;
}

interface Snapshot {
  state: GameState;
  status: GameStatus;
  lastMove: Move | null;
  legal: Move[];
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

export function createGameController(engine: EngineClient, config: ControllerConfig = {}): GameController {
  const cpuDelay = config.cpuDelayMs ?? 400;
  const listeners = new Set<(v: GameView) => void>();

  let state: GameState | null = null;
  let status: GameStatus = { type: "playing" };
  let humanSide: Side = "sente";
  let level: Level = 1;
  let selection: Selection = { type: "none" };
  let lastMove: Move | null = null;
  let thinking = false;
  let legal: Move[] = []; // 人間手番時の合法手キャッシュ
  let undoStack: Snapshot[] = [];
  let busy = false; // 人間の手の処理中（再入防止）
  let error: string | null = null;
  let gameId = 0; // newGame で無効化するための世代番号

  const emptyState = (): GameState => ({
    board: Array(12).fill(null),
    hands: {
      sente: { giraffe: 0, elephant: 0, chick: 0 },
      gote: { giraffe: 0, elephant: 0, chick: 0 },
    },
    turn: "sente",
    history: [],
  });

  function computeTargets(): Square[] {
    if (selection.type === "none") return [];
    const set = new Set<Square>();
    for (const m of legal) {
      if (selection.type === "square" && m.type === "move" && m.from === selection.square) set.add(m.to);
      if (selection.type === "hand" && m.type === "drop" && m.kind === selection.kind) set.add(m.to);
    }
    return [...set].sort((a, b) => a - b);
  }

  function getView(): GameView {
    return {
      state: state ?? emptyState(),
      status,
      humanSide,
      level,
      selection,
      targets: computeTargets(),
      lastMove,
      thinking,
      error,
      canUndo: !thinking && !busy && status.type === "playing" && undoStack.length > 0,
    };
  }

  function emit() {
    const v = getView();
    for (const l of [...listeners]) l(v);
  }

  const isHumanTurn = () =>
    state !== null && !thinking && !busy && status.type === "playing" && state.turn === humanSide;

  async function refreshLegal() {
    legal = state && status.type === "playing" ? await engine.legalMoves(state) : [];
  }

  /** CPU 手番なら指す。gameId が変わったら中断。 */
  async function cpuTurnIfNeeded(id: number) {
    if (!state || status.type !== "playing" || state.turn === humanSide) return;
    thinking = true;
    error = null;
    selection = { type: "none" };
    legal = [];
    emit();
    try {
      const started = Date.now();
      const res = await engine.chooseMove(state, level, { seed: Math.floor(Math.random() * 0x7fffffff) });
      const wait = cpuDelay - (Date.now() - started);
      if (wait > 0) await sleep(wait);
      if (id !== gameId) return;
      const applied = await engine.applyMove(state, res.move);
      if (id !== gameId) return;
      if (!applied.ok) throw new Error(applied.error);
      state = applied.value;
      lastMove = res.move;
      status = await engine.gameStatus(state);
      if (id !== gameId) return;
      await refreshLegal();
    } catch (e) {
      if (id === gameId) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (id === gameId) {
        thinking = false;
        emit();
      }
    }
  }

  async function newGame(options: NewGameOptions) {
    const id = ++gameId;
    humanSide = options.humanSide;
    level = options.level;
    selection = { type: "none" };
    lastMove = null;
    undoStack = [];
    thinking = false;
    busy = false;
    error = null;
    legal = [];
    state = null;
    status = { type: "playing" };
    emit();
    try {
      state = await engine.initialState();
      if (id !== gameId) return;
      status = await engine.gameStatus(state);
      if (id !== gameId) return;
      await refreshLegal();
    } catch (e) {
      if (id === gameId) {
        error = e instanceof Error ? e.message : String(e);
        emit();
      }
      return;
    }
    emit();
    await cpuTurnIfNeeded(id);
  }

  async function clickSquare(square: Square) {
    if (!isHumanTurn() || !state) return;
    const id = gameId;
    const targets = computeTargets();

    if (selection.type !== "none" && targets.includes(square)) {
      const move: Move =
        selection.type === "square"
          ? { type: "move", from: selection.square, to: square }
          : { type: "drop", kind: selection.kind, to: square };
      const before = state;
      busy = true;
      try {
        const applied = await engine.applyMove(before, move);
        if (id !== gameId) return;
        if (!applied.ok) {
          selection = { type: "none" };
          emit();
          return;
        }
        undoStack.push({ state: before, status, lastMove, legal });
        state = applied.value;
        lastMove = move;
        selection = { type: "none" };
        status = await engine.gameStatus(state);
        if (id !== gameId) return;
        await refreshLegal();
      } catch (e) {
        if (id === gameId) {
          error = e instanceof Error ? e.message : String(e);
          emit();
        }
        return;
      } finally {
        if (id === gameId) busy = false;
      }
      emit();
      await cpuTurnIfNeeded(id);
      return;
    }

    const piece = state.board[square];
    if (piece && piece.owner === humanSide) {
      selection =
        selection.type === "square" && selection.square === square
          ? { type: "none" }
          : { type: "square", square };
    } else {
      selection = { type: "none" };
    }
    emit();
  }

  function clickHand(kind: HandKind) {
    if (!isHumanTurn() || !state) return;
    if (state.hands[humanSide][kind] <= 0) return;
    selection =
      selection.type === "hand" && selection.kind === kind ? { type: "none" } : { type: "hand", kind };
    emit();
  }

  function clearSelection() {
    if (selection.type === "none") return;
    selection = { type: "none" };
    emit();
  }

  function undo() {
    if (thinking || busy || status.type !== "playing") return;
    const snap = undoStack.pop();
    if (!snap) return;
    gameId++; // 進行中の非同期処理を無効化
    state = snap.state;
    status = snap.status;
    lastMove = snap.lastMove;
    legal = snap.legal;
    selection = { type: "none" };
    error = null;
    emit();
  }

  async function retry() {
    if (thinking || busy || !error) return;
    await cpuTurnIfNeeded(gameId);
  }

  return {
    getView,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    newGame,
    clickSquare,
    clickHand,
    clearSelection,
    undo,
    retry,
  };
}
