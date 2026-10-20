/// <reference lib="webworker" />
// WASM をロードして実行する Worker (docs/spec.md §5.1)。
// 受信: {id, method, args} / 送信: {id, result} | {id, error} と、初期化完了通知 {ready: true} | {ready: false, error}。
import init, {
  initialState,
  legalMoves,
  applyMove,
  gameStatus,
  chooseMove,
} from "../wasm/pkg/animal_shogi_engine.js";

interface Request {
  id: number;
  method: string;
  args: unknown[];
}

const json = (v: unknown): string => JSON.stringify(v);

/** 各メソッドは Result 封筒（パース済み）を result として返す。 */
const handlers: Record<string, (...args: any[]) => string> = {
  initialState: () => initialState(),
  legalMoves: (state) => legalMoves(json(state)),
  applyMove: (state, move) => applyMove(json(state), json(move)),
  gameStatus: (state) => gameStatus(json(state)),
  chooseMove: (state, level: number, maxNodes: number, seed: number) =>
    chooseMove(json(state), level, maxNodes, seed),
};

init().then(
  () => {
    self.postMessage({ ready: true });
  },
  (e: unknown) => {
    self.postMessage({ ready: false, error: String(e) });
  },
);

self.onmessage = (ev: MessageEvent<Request>) => {
  const { id, method, args } = ev.data;
  try {
    const h = handlers[method];
    if (!h) throw new Error(`unknownMethod: ${method}`);
    self.postMessage({ id, result: JSON.parse(h(...args)) });
  } catch (e) {
    self.postMessage({ id, error: e instanceof Error ? e.message : String(e) });
  }
};
