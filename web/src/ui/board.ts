import type { GameController, GameView } from "../game/controller";
import type { HandKind, Piece, PieceKind, Side, Square } from "../types";

const EMOJI: Record<PieceKind, string> = {
  lion: "🦁", giraffe: "🦒", elephant: "🐘", chick: "🐥", hen: "🐔",
};
const NAME: Record<PieceKind, string> = {
  lion: "ライオン", giraffe: "キリン", elephant: "ゾウ", chick: "ヒヨコ", hen: "ニワトリ",
};
const HAND_KINDS: HandKind[] = ["giraffe", "elephant", "chick"];
const SIDE_NAME: Record<Side, string> = { sente: "先手", gote: "後手" };

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls = "", text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

function pieceEl(p: Piece, human: Side): HTMLElement {
  const e = el("span", `piece kind-${p.kind}${p.owner === human ? "" : " opp"}`, EMOJI[p.kind]);
  e.setAttribute("role", "img");
  e.setAttribute("aria-label", `${SIDE_NAME[p.owner]}の${NAME[p.kind]}`);
  return e;
}

function handArea(view: GameView, side: Side, ctl: GameController): HTMLElement {
  const area = el("div", `hand hand-${side}`);
  area.append(el("span", "hand-label", `${SIDE_NAME[side]}${side === view.humanSide ? "(あなた)" : "(CPU)"}`));
  for (const kind of HAND_KINDS) {
    const n = view.state.hands[side][kind];
    const b = el("button", "hand-slot");
    b.type = "button";
    b.append(pieceEl({ kind, owner: side }, view.humanSide), el("span", "count", `×${n}`));
    const mine = side === view.humanSide;
    const selected = mine && view.selection.type === "hand" && view.selection.kind === kind;
    if (selected) b.classList.add("selected");
    if (n === 0) b.classList.add("empty");
    b.disabled = !mine || n === 0;
    b.addEventListener("click", () => ctl.clickHand(kind));
    area.append(b);
  }
  return area;
}

function boardEl(view: GameView, ctl: GameController): HTMLElement {
  const board = el("div", "board");
  const last = view.lastMove;
  const lastSquares = new Set<Square>();
  if (last) {
    lastSquares.add(last.to);
    if (last.type === "move") lastSquares.add(last.from);
  }
  // 人間が後手のときは盤を 180° 回して自分を下側に表示する
  const flip = view.humanSide === "gote";
  for (let i = 0; i < 12; i++) {
    const sq = flip ? 11 - i : i;
    const cell = el("button", "cell");
    cell.type = "button";
    if (view.selection.type === "square" && view.selection.square === sq) cell.classList.add("selected");
    if (view.targets.includes(sq)) cell.classList.add("target");
    if (lastSquares.has(sq)) cell.classList.add("last");
    const p = view.state.board[sq];
    if (p) cell.append(pieceEl(p, view.humanSide));
    cell.addEventListener("click", () => void ctl.clickSquare(sq));
    board.append(cell);
  }
  return board;
}

function statusText(view: GameView): string {
  if (view.status.type === "won") return `${SIDE_NAME[view.status.winner]}の勝ち`;
  if (view.status.type === "draw") return "引き分け";
  if (view.thinking) return "CPU が考え中…";
  return view.state.turn === view.humanSide ? "あなたの番です" : "CPU の番です";
}

const REASON: Record<string, string> = {
  captureLion: "ライオンを捕まえました",
  try: "トライ成功(ライオンが敵陣に到達)",
  noMoves: "指せる手がありません",
  repetition: "千日手です",
};

export function mountUI(root: HTMLElement, ctl: GameController, initial: { humanSide: Side; level: 1 | 2 | 3 }) {
  let humanSide = initial.humanSide;
  let level = initial.level;

  const render = (view: GameView) => {
    root.replaceChildren();
    const app = el("div", "app");
    app.append(el("h1", "title", "どうぶつしょうぎ"));

    const st = el("div", "status", statusText(view));
    if (view.thinking) st.classList.add("thinking");
    app.append(st);

    const opponent: Side = view.humanSide === "sente" ? "gote" : "sente";
    app.append(handArea(view, opponent, ctl), boardEl(view, ctl), handArea(view, view.humanSide, ctl));

    const settings = el("div", "settings");
    const sideSel = el("select");
    for (const s of ["sente", "gote"] as Side[]) {
      const o = el("option", "", `${SIDE_NAME[s]}`);
      o.value = s;
      o.selected = s === humanSide;
      sideSel.append(o);
    }
    sideSel.addEventListener("change", () => (humanSide = sideSel.value as Side));
    const lvSel = el("select");
    for (const [v, t] of [[1, "よわい"], [2, "ふつう"], [3, "つよい"]] as const) {
      const o = el("option", "", `レベル${v} ${t}`);
      o.value = String(v);
      o.selected = v === level;
      lvSel.append(o);
    }
    lvSel.addEventListener("change", () => (level = Number(lvSel.value) as 1 | 2 | 3));
    const start = el("button", "btn primary", "新しい対局");
    start.type = "button";
    start.addEventListener("click", () => void ctl.newGame({ humanSide, level }));
    const undo = el("button", "btn", "やり直し");
    undo.type = "button";
    undo.disabled = !view.canUndo;
    undo.addEventListener("click", () => ctl.undo());
    settings.append(sideSel, lvSel, start, undo);
    app.append(settings);

    if (view.status.type !== "playing") {
      const overlay = el("div", "overlay");
      const dlg = el("div", "dialog");
      dlg.setAttribute("role", "dialog");
      const humanWon = view.status.type === "won" && view.status.winner === view.humanSide;
      const title =
        view.status.type === "draw" ? "引き分け" : humanWon ? "あなたの勝ち!" : "あなたの負け…";
      dlg.append(el("h2", "", title), el("p", "", statusText(view)), el("p", "reason", REASON[view.status.reason]));
      const again = el("button", "btn primary", "もう一度遊ぶ");
      again.type = "button";
      again.addEventListener("click", () => void ctl.newGame({ humanSide, level }));
      dlg.append(again);
      overlay.append(dlg);
      app.append(overlay);
    }
    root.append(app);
  };

  ctl.subscribe(render);
  render(ctl.getView());
}
