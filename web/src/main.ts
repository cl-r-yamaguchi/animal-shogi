import "./style.css";
import { createGameController } from "./game/controller";
import { createMockEngine } from "./game/mockEngine";
import { mountUI } from "./ui/board";
import { createEngineClient } from "./engine/client";
import type { EngineClient } from "./types";

async function main() {
  const root = document.getElementById("app")!;
  root.textContent = "読み込み中…";
  // ?mock=1 で WASM なしのモックエンジンを使う（UI の単独確認用）
  const useMock = new URLSearchParams(location.search).get("mock") === "1";
  let engine: EngineClient;
  try {
    engine = useMock ? createMockEngine() : await createEngineClient();
  } catch (e) {
    root.textContent = `エンジンの初期化に失敗しました: ${e instanceof Error ? e.message : String(e)}`;
    return;
  }
  const ctl = createGameController(engine);
  mountUI(root, ctl, { humanSide: "sente", level: 2 });
  await ctl.newGame({ humanSide: "sente", level: 2 });
}

void main();
