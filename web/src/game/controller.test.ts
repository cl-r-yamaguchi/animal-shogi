import { describe, expect, it } from "vitest";
import { createGameController } from "./controller";
import { createMockEngine, mockApply, mockInitialState } from "./mockEngine";
import type { EngineClient, GameState, Move } from "../types";

const make = (engine: EngineClient = createMockEngine()) =>
  createGameController(engine, { cpuDelayMs: 0 });

describe("mockEngine", () => {
  it("初期局面の合法手は共通テストベクタの4手", () => {
    const e = createMockEngine();
    return e.legalMoves(mockInitialState()).then((m) => {
      expect(m).toEqual([
        { type: "move", from: 7, to: 4 },
        { type: "move", from: 10, to: 6 },
        { type: "move", from: 10, to: 8 },
        { type: "move", from: 11, to: 8 },
      ]);
    });
  });
});

describe("GameController", () => {
  it("newGame(先手)で初期局面・CPU は動かない", async () => {
    const c = make();
    await c.newGame({ humanSide: "sente", level: 1 });
    const v = c.getView();
    expect(v.state.turn).toBe("sente");
    expect(v.thinking).toBe(false);
    expect(v.canUndo).toBe(false);
    expect(v.lastMove).toBeNull();
  });

  it("後手選択時は CPU が初手を指す", async () => {
    const c = make();
    await c.newGame({ humanSide: "gote", level: 2 });
    const v = c.getView();
    expect(v.state.turn).toBe("gote");
    expect(v.lastMove).not.toBeNull();
    expect(v.state.history).toHaveLength(2);
    expect(v.thinking).toBe(false);
  });

  it("選択・切替・解除と targets", async () => {
    const c = make();
    await c.newGame({ humanSide: "sente", level: 1 });
    await c.clickSquare(7);
    expect(c.getView().selection).toEqual({ type: "square", square: 7 });
    expect(c.getView().targets).toEqual([4]);
    await c.clickSquare(10);
    expect(c.getView().selection).toEqual({ type: "square", square: 10 });
    expect(c.getView().targets).toEqual([6, 8]);
    await c.clickSquare(10);
    expect(c.getView().selection).toEqual({ type: "none" });
    await c.clickSquare(4); // 相手駒・未選択 → 無視
    expect(c.getView().selection).toEqual({ type: "none" });
  });

  it("移動すると CPU が応手し、通知が飛ぶ", async () => {
    const c = make();
    await c.newGame({ humanSide: "sente", level: 1 });
    const views: boolean[] = [];
    const off = c.subscribe((v) => views.push(v.thinking));
    await c.clickSquare(10);
    await c.clickSquare(6);
    off();
    expect(views).toContain(true);
    const v = c.getView();
    expect(v.state.history).toHaveLength(3);
    expect(v.state.turn).toBe("sente");
    expect(v.canUndo).toBe(true);
    expect(v.thinking).toBe(false);
  });

  it("thinking 中の入力は無視される", async () => {
    const base = createMockEngine();
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    const c = make({ ...base, chooseMove: async (s, l, o) => { await gate; return base.chooseMove(s, l, o); } });
    await c.newGame({ humanSide: "sente", level: 1 });
    await c.clickSquare(7);
    const p = c.clickSquare(4);
    await new Promise((r) => setTimeout(r, 0));
    expect(c.getView().thinking).toBe(true);
    await c.clickSquare(10);
    c.clickHand("chick");
    expect(c.getView().selection).toEqual({ type: "none" });
    c.undo();
    release();
    await p;
    expect(c.getView().thinking).toBe(false);
  });

  it("持ち駒を打てる", async () => {
    const c = make();
    await c.newGame({ humanSide: "sente", level: 1 });
    // ヒヨコで相手ヒヨコを取る → 持ち駒 chick 1
    await c.clickSquare(7);
    await c.clickSquare(4);
    const v0 = c.getView();
    expect(v0.state.hands.sente.chick).toBe(1);
    c.clickHand("chick");
    expect(c.getView().selection).toEqual({ type: "hand", kind: "chick" });
    const t = c.getView().targets;
    expect(t.length).toBeGreaterThan(0);
    const before = c.getView().state.board.filter(Boolean).length;
    await c.clickSquare(t[0]);
    expect(c.getView().state.board.filter(Boolean).length).toBeGreaterThanOrEqual(before);
    c.clickHand("giraffe"); // 持っていない
    expect(c.getView().selection).toEqual({ type: "none" });
  });

  it("undo で人間の手と CPU の応手を巻き戻す", async () => {
    const c = make();
    await c.newGame({ humanSide: "sente", level: 1 });
    const initial = c.getView().state;
    await c.clickSquare(10);
    await c.clickSquare(6);
    c.undo();
    const v = c.getView();
    expect(v.state).toEqual(initial);
    expect(v.canUndo).toBe(false);
    expect(v.lastMove).toBeNull();
    // 巻き戻し後も操作できる
    await c.clickSquare(7);
    expect(c.getView().targets).toEqual([4]);
  });

  it("後手選択での undo は CPU 初手の後の局面まで戻る", async () => {
    const c = make();
    await c.newGame({ humanSide: "gote", level: 1 });
    const afterCpu = c.getView().state;
    // モックの CPU 初手は 7→4 (ヒヨコ取り)。後手ライオン(1)で取り返す
    await c.clickSquare(1);
    await c.clickSquare(4);
    expect(c.getView().state.history).toHaveLength(4);
    c.undo();
    expect(c.getView().state).toEqual(afterCpu);
  });

  it("終局: 人間がライオンを取ると勝ち、以後の入力・undo は無視", async () => {
    // 先手ライオン(4)の隣に後手ライオン(1)がいる局面から開始
    const s: GameState = { ...mockInitialState(), board: Array(12).fill(null) };
    s.board[4] = { kind: "lion", owner: "sente" };
    s.board[1] = { kind: "lion", owner: "gote" };
    s.board[11] = { kind: "giraffe", owner: "gote" };
    s.history = ["x"];
    const base = createMockEngine();
    const c = make({ ...base, initialState: async () => s });
    await c.newGame({ humanSide: "sente", level: 1 });
    await c.clickSquare(4);
    await c.clickSquare(1);
    const v = c.getView();
    expect(v.status).toEqual({ type: "won", winner: "sente", reason: "captureLion" });
    expect(v.thinking).toBe(false);
    expect(v.canUndo).toBe(false);
    await c.clickSquare(1);
    expect(c.getView().selection).toEqual({ type: "none" });
  });

  it("CPU が勝って終局する", async () => {
    const s: GameState = { ...mockInitialState(), board: Array(12).fill(null) };
    s.board[7] = { kind: "lion", owner: "sente" };
    s.board[8] = { kind: "chick", owner: "sente" }; // 人間の動かせる駒
    s.board[4] = { kind: "lion", owner: "gote" };
    s.history = ["x"];
    const c = make({ ...createMockEngine(), initialState: async () => s });
    await c.newGame({ humanSide: "sente", level: 1 });
    await c.clickSquare(8);
    await c.clickSquare(5);
    // 後手(CPU)は先手ライオン(7)を取る(4→7)
    expect(c.getView().status).toEqual({ type: "won", winner: "gote", reason: "captureLion" });
  });

  it("newGame で進行中の CPU 思考を無効化する", async () => {
    const base = createMockEngine();
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    let first = true;
    const c = make({
      ...base,
      chooseMove: async (s, l, o) => {
        if (first) { first = false; await gate; }
        return base.chooseMove(s, l, o);
      },
    });
    const p = c.newGame({ humanSide: "gote", level: 1 });
    await new Promise((r) => setTimeout(r, 0));
    await c.newGame({ humanSide: "sente", level: 1 });
    release();
    await p;
    expect(c.getView().state.history).toHaveLength(1);
    expect(c.getView().humanSide).toBe("sente");
  });
});

describe("mockApply", () => {
  it("ヒヨコは最奥段でニワトリになる", () => {
    const s = mockInitialState();
    s.board = Array(12).fill(null);
    s.board[4] = { kind: "chick", owner: "sente" };
    s.board[11] = { kind: "lion", owner: "sente" };
    s.board[9] = { kind: "lion", owner: "gote" };
    const mv: Move = { type: "move", from: 4, to: 1 };
    const r = mockApply(s, mv);
    expect(r.ok && r.value.board[1]?.kind).toBe("hen");
  });
});
