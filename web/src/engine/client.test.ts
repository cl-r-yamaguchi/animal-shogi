import { describe, expect, it } from "vitest";
import { createEngineClient, type WorkerLike } from "./client";

/** Worker のモック。受信メッセージを記録し、handler の戻り値を応答する。 */
function mockWorker(handler: (msg: any) => any, opts: { init?: any } = {}) {
  const sent: any[] = [];
  let terminated = false;
  const w: WorkerLike = {
    onmessage: null,
    onerror: null,
    postMessage(msg: any) {
      sent.push(msg);
      queueMicrotask(() => w.onmessage?.({ data: { id: msg.id, ...handler(msg) } }));
    },
    terminate() {
      terminated = true;
    },
  };
  queueMicrotask(() => w.onmessage?.({ data: opts.init ?? { ready: true } }));
  return { w, sent, isTerminated: () => terminated };
}

const state: any = { board: [], hands: {}, turn: "sente", history: [] };

describe("createEngineClient", () => {
  it("WASM 初期化の完了後に解決し、メッセージ形式 {id, method, args} で送る", async () => {
    const m = mockWorker(() => ({ result: { ok: true, value: [{ type: "move", from: 7, to: 4 }] } }));
    const c = await createEngineClient(() => m.w);
    const moves = await c.legalMoves(state);
    expect(moves).toEqual([{ type: "move", from: 7, to: 4 }]);
    expect(m.sent[0]).toEqual({ id: expect.any(Number), method: "legalMoves", args: [state] });
  });

  it("初期化失敗なら reject する", async () => {
    const m = mockWorker(() => ({}), { init: { ready: false, error: "boom" } });
    await expect(createEngineClient(() => m.w)).rejects.toThrow("boom");
  });

  it("ok:false は reject (message は error 文字列)", async () => {
    const m = mockWorker(() => ({ result: { ok: false, error: "invalidJson" } }));
    const c = await createEngineClient(() => m.w);
    await expect(c.gameStatus(state)).rejects.toThrow("invalidJson");
  });

  it("applyMove は Result をそのまま返す", async () => {
    const m = mockWorker(() => ({ result: { ok: false, error: "illegalMovement" } }));
    const c = await createEngineClient(() => m.w);
    const r = await c.applyMove(state, { type: "move", from: 1, to: 2 });
    expect(r).toEqual({ ok: false, error: "illegalMovement" });
  });

  it("chooseMove は level/maxNodes/seed を渡し、noMoves は reject", async () => {
    const m = mockWorker(() => ({ result: { ok: false, error: "noMoves" } }));
    const c = await createEngineClient(() => m.w);
    await expect(c.chooseMove(state, 2, { maxNodes: 500, seed: 7 })).rejects.toThrow("noMoves");
    expect(m.sent[0].method).toBe("chooseMove");
    expect(m.sent[0].args).toEqual([state, 2, 500, 7]);
    await expect(c.chooseMove(state, 1)).rejects.toThrow("noMoves");
    expect(m.sent[1].args.slice(0, 3)).toEqual([state, 1, 0]);
    expect(typeof m.sent[1].args[3]).toBe("number");
  });

  it("Worker からの error メッセージは reject になり、並行リクエストを id で対応付ける", async () => {
    const m = mockWorker((msg) =>
      msg.method === "initialState" ? { error: "unknownMethod" } : { result: { ok: true, value: { type: "playing" } } },
    );
    const c = await createEngineClient(() => m.w);
    const [a, b] = await Promise.allSettled([c.initialState(), c.gameStatus(state)]);
    expect(a.status).toBe("rejected");
    expect(b).toEqual({ status: "fulfilled", value: { type: "playing" } });
  });

  it("dispose は Worker を終了し、以降の呼び出しは reject する", async () => {
    const m = mockWorker(() => ({ result: { ok: true, value: null } }));
    const c = await createEngineClient(() => m.w);
    c.dispose();
    expect(m.isTerminated()).toBe(true);
    await expect(c.initialState()).rejects.toThrow("disposed");
  });
});
