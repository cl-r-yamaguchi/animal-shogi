// エンジンクライアント (docs/spec.md §5.1)。WASM は Worker 内で実行する。
import type {
  AiOptions,
  AiResult,
  EngineClient,
  GameState,
  GameStatus,
  Move,
  MoveError,
  Result,
} from "../types";

/** Worker 互換の最小インターフェース（テストでモックできるようにする）。 */
export interface WorkerLike {
  postMessage(message: unknown): void;
  terminate(): void;
  onmessage: ((ev: { data: any }) => void) | null;
  onerror: ((ev: any) => void) | null;
}

export type WorkerFactory = () => WorkerLike;

const defaultWorkerFactory: WorkerFactory = () =>
  new Worker(new URL("./worker.ts", import.meta.url), { type: "module" }) as unknown as WorkerLike;

interface Pending {
  resolve: (v: any) => void;
  reject: (e: Error) => void;
}

/** Worker を起動し、WASM 初期化の完了後に解決する。 */
export function createEngineClient(
  workerFactory: WorkerFactory = defaultWorkerFactory,
): Promise<EngineClient> {
  const worker = workerFactory();
  const pending = new Map<number, Pending>();
  let nextId = 1;
  let disposed = false;

  const rejectAll = (e: Error) => {
    for (const p of pending.values()) p.reject(e);
    pending.clear();
  };

  return new Promise<EngineClient>((resolveClient, rejectClient) => {
    let ready = false;

    /** Result 封筒を呼び出し、生の Result を返す。 */
    const callRaw = <T, E = string>(method: string, args: unknown[]): Promise<Result<T, E>> => {
      if (disposed) return Promise.reject(new Error("disposed"));
      const id = nextId++;
      return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        worker.postMessage({ id, method, args });
      });
    };

    /** ok:false は reject にする。 */
    const call = async <T>(method: string, args: unknown[]): Promise<T> => {
      const r = await callRaw<T>(method, args);
      if (r.ok) return r.value;
      throw new Error(r.error);
    };

    const client: EngineClient = {
      initialState: () => call<GameState>("initialState", []),
      legalMoves: (state: GameState) => call<Move[]>("legalMoves", [state]),
      applyMove: (state: GameState, move: Move) =>
        callRaw<GameState, MoveError>("applyMove", [state, move]),
      gameStatus: (state: GameState) => call<GameStatus>("gameStatus", [state]),
      chooseMove: (state: GameState, level: 1 | 2 | 3, opts?: AiOptions) =>
        call<AiResult>("chooseMove", [
          state,
          level,
          opts?.maxNodes ?? 0,
          // seed 省略時は乱数
          opts?.seed ?? Math.floor(Math.random() * 0x100000000),
        ]),
      dispose() {
        if (disposed) return;
        disposed = true;
        worker.terminate();
        rejectAll(new Error("disposed"));
      },
    };

    worker.onmessage = (ev) => {
      const data = ev.data;
      if (data && typeof data === "object" && "ready" in data) {
        if (data.ready) {
          ready = true;
          resolveClient(client);
        } else {
          worker.terminate();
          rejectClient(new Error(data.error ?? "wasm init failed"));
        }
        return;
      }
      if (!data || typeof data !== "object") return;
      const p = pending.get(data.id);
      if (!p) return;
      pending.delete(data.id);
      if ("error" in data) p.reject(new Error(String(data.error)));
      else p.resolve(data.result);
    };

    worker.onerror = (ev) => {
      const e = new Error(ev?.message ?? "worker error");
      // 以降の呼び出しはハングさせず即 reject する
      disposed = true;
      worker.terminate();
      if (!ready) rejectClient(e);
      rejectAll(e);
    };
  });
}
