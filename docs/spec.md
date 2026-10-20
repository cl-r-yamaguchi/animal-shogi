# どうぶつしょうぎ 設計書 (spec)

ブラウザで遊べる「人間 vs コンピュータ」のどうぶつしょうぎ。
ルール判定と AI は **Rust (WASM)**、UI と画面制御は **TypeScript (Vite, フレームワークなし)** で実装する。

この文書は並列実装の契約である。**ここに書かれた名前・型・JSON 形式・ファイル所有権は、変更せずに守ること。** 変更が必要なら実装前にリードへ相談する。

---

## 1. 正式ルール

### 1.1 盤と駒
- 盤は **3 列 × 4 段**。
- 先手 (`sente`) は下側、後手 (`gote`) は上側。**先手から交互に指す**。
- 人間はデフォルトで先手。UI で先手/後手を選べる。

| 駒 | `PieceKind` | 動き |
|---|---|---|
| ライオン | `lion` | 縦横斜め 8 方向に 1 マス |
| キリン | `giraffe` | 縦横 4 方向に 1 マス |
| ゾウ | `elephant` | 斜め 4 方向に 1 マス |
| ヒヨコ | `chick` | 前に 1 マス |
| ニワトリ | `hen` | 前・斜め前 2・横 2・後ろの 5 方向に 1 マス（金将の動き） |

「前」は先手なら画面上方向、後手なら下方向。

### 1.2 初期配置
```
      列0  列1  列2
段0   キリン ライオン ゾウ      ← 後手 (向きは反転して表示)
段1    .   ヒヨコ    .
段2    .   ヒヨコ    .
段3   ゾウ  ライオン キリン     ← 先手
```
初期局面の `positionKey` は `gle.c..C.ELG/000/000/s`（§2.4）。

### 1.3 指し方
1. **移動**: 自分の駒を、動ける範囲の空きマスまたは相手の駒があるマスへ動かす。相手の駒があれば取る。自分の駒があるマスへは動けない。
2. **取った駒は持ち駒**になり、自分のものとして使える。取ったニワトリは**ヒヨコに戻って**持ち駒になる。取ったライオンは持ち駒にならない（その時点でゲーム終了）。
3. **打ち**: 手番では、移動の代わりに持ち駒 1 枚を**任意の空きマス**に打てる（キリン・ゾウ・ヒヨコ）。打った駒は常に未成の状態で、向きは打った側のものになる。
   - 二歩・打ち歩詰めのような**打ち制限はない**。最奥段にヒヨコを打ってもよい（動けず、成れないまま置かれる）。
4. **成り**: ヒヨコが**敵陣最奥段**（先手は段 0、後手は段 3）に**移動して着いたとき、自動的に（強制で）ニワトリに成る**。打って最奥段に置いた場合は成らない。
5. **王手の概念はない**。取られる位置にライオンを動かすことも合法である。**全ての擬似合法手が合法手**である。

### 1.4 勝敗
手番側が `S`、直前に指した側が `P`（`= 相手(S)`）としたとき、局面から次の順で判定する（`gameStatus` の仕様）。

1. **キャッチ**: 盤上にライオンが 1 体しかない場合、残っている側の勝ち（`reason: "captureLion"`）。
2. **トライ**: 手番側 `S` のライオンが、`S` から見た敵陣最奥段（先手: 段 0、後手: 段 3）にいる場合、`S` の勝ち（`reason: "try"`）。
   - つまり「最奥段に到達し、相手の次の 1 手で取られなかった」ときに勝ち。到達した直後（相手の手番）は勝ちではない。
   - 到達した側の手番でこれが成立することはない（先に相手が指すため）。
3. **詰み相当**: 手番側に合法手がない場合、手番側の負け（`reason: "noMoves"`）。通常は到達しないが、定義しておく。
4. **千日手**: 同一の `positionKey` が履歴中に **3 回** 現れたら引き分け（`reason: "repetition"`）。
5. 上記のいずれでもなければ `playing`。

ゲーム終了後、`legalMoves` は空配列を返し、`applyMove` は `gameOver` エラーを返す。

---

## 2. データ構造

Rust と TypeScript の間は **JSON 文字列** でやり取りする。フィールド名は camelCase、列挙値は下記の文字列リテラルで厳密に一致させる。

### 2.1 基本型

```ts
type Side = "sente" | "gote";
type PieceKind = "lion" | "giraffe" | "elephant" | "chick" | "hen";
type HandKind = "giraffe" | "elephant" | "chick";   // 持ち駒にできる駒

interface Piece { kind: PieceKind; owner: Side; }

/** マス番号 0..=11。sq = row * 3 + col。row 0 が画面最上段、col 0 が画面左端。 */
type Square = number;
```

```
     col0 col1 col2
row0   0    1    2
row1   3    4    5
row2   6    7    8
row3   9   10   11
```

### 2.2 局面とゲーム状態

```ts
type Hand = Record<HandKind, number>;          // 各 0..=2

interface GameState {
  board: (Piece | null)[];                     // 長さ 12、添字が Square
  hands: Record<Side, Hand>;
  turn: Side;                                  // 手番
  history: string[];                           // これまでの全局面の positionKey（初期局面と現局面を含む）
}
```

- `history` の末尾は常に現局面の `positionKey`。`applyMove` が更新する。

### 2.3 手

```ts
type Move =
  | { type: "move"; from: Square; to: Square }          // 成りは自動なので指定しない
  | { type: "drop"; kind: HandKind; to: Square };
```

`legalMoves` の返却順は決定的（テストで比較できるように）:
1. `move` を `from` 昇順、同一 `from` なら `to` 昇順
2. 続けて `drop` を `kind` の順 (`giraffe` → `elephant` → `chick`)、同一 `kind` なら `to` 昇順

### 2.4 positionKey

局面の同一性判定用の文字列。`"{盤12文字}/{先手持ち駒}/{後手持ち駒}/{手番}"`。

- 盤: マス 0〜11 の順に 1 文字。空 `.`、`l`/`g`/`e`/`c`/`h` = lion/giraffe/elephant/chick/hen。**先手は大文字、後手は小文字**。
- 持ち駒: `giraffe, elephant, chick` の枚数を 3 桁の数字で（例 `010`）。
- 手番: `s` または `g`。
- 例（初期局面）: `gle.c..C.ELG/000/000/s`

### 2.5 結果・エラー

```ts
type GameStatus =
  | { type: "playing" }
  | { type: "won"; winner: Side; reason: "captureLion" | "try" | "noMoves" }
  | { type: "draw"; reason: "repetition" };

type MoveError =
  | "gameOver" | "invalidSquare" | "noPiece" | "notOwnPiece"
  | "illegalMovement" | "occupiedByOwn" | "noPieceInHand" | "squareOccupied";

type Result<T, E = string> = { ok: true; value: T } | { ok: false; error: E };

interface AiResult {
  move: Move;
  score: number;     // 手番側から見た評価値。勝ち確定は +10000 付近、負け確定は -10000 付近
  depth: number;     // 到達した探索深さ（level 1 では 0）
  nodes: number;     // 探索ノード数
}
```

### 2.6 駒の動き表（実装共通）

方向は `(dRow, dCol)`。先手の「前」は `dRow = -1`、後手は `dRow = +1`。後手は下表の `dRow` の符号を反転する（`dCol` は反転しない。左右対称なので不要）。

| 駒 | 先手から見た移動 |
|---|---|
| lion | (-1,-1) (-1,0) (-1,1) (0,-1) (0,1) (1,-1) (1,0) (1,1) |
| giraffe | (-1,0) (0,-1) (0,1) (1,0) |
| elephant | (-1,-1) (-1,1) (1,-1) (1,1) |
| chick | (-1,0) |
| hen | (-1,-1) (-1,0) (-1,1) (0,-1) (0,1) (1,0) |

盤外へは動けない。

---

## 3. 構成とファイル所有権

```
animal-shogi/
├─ docs/spec.md                      [リード]
├─ engine/                           Rust クレート (名前: animal_shogi_engine)
│   ├─ Cargo.toml                    [リード]
│   ├─ src/lib.rs                    [リード]  mod 宣言と公開 re-export のみ
│   ├─ src/types.rs                  [リード]  §4.1 の型 (serde 付き)
│   ├─ src/rules.rs                  [担当A]
│   ├─ src/wasm.rs                   [担当A]
│   ├─ tests/rules_test.rs           [担当A]
│   ├─ src/ai.rs                     [担当B]   (必要なら src/ai/*.rs に分割可)
│   ├─ tests/ai_test.rs              [担当B]
│   └─ examples/selfplay.rs          [担当B]
└─ web/                              Vite + TypeScript (フレームワークなし) + Vitest
    ├─ package.json, tsconfig.json, vite.config.ts, index.html   [リード]
    ├─ src/types.ts                  [リード]  §2 の TS 型 + §5.1 の EngineClient 型
    ├─ src/engine/client.ts          [担当A]
    ├─ src/engine/worker.ts          [担当A]
    ├─ src/engine/client.test.ts     [担当A]
    ├─ src/wasm/pkg/                 wasm-pack 生成物 (git 管理外)
    ├─ src/game/**                   [担当C]   コントローラ
    ├─ src/ui/**                     [担当C]   描画・入力
    ├─ src/main.ts                   [担当C]
    └─ src/style.css                 [担当C]
```

### 所有権のルール
- **自分の担当ファイル以外は編集しない。** 共有ファイル（`[リード]`）に変更が必要なら、リードに依頼する。
- 担当ごとに依存する相手の実装が未完でも、**§4・§5 のシグネチャだけを前提に**進める。
- 担当 C は、`EngineClient` を差し替え可能にして依存し、`src/game/mockEngine.ts`（担当 C 所有、固定応答でよい）で単独開発できるようにする。
- ビルド:
  - Rust: `cd engine && cargo test`
  - WASM: `cd engine && wasm-pack build --target web --out-dir ../web/src/wasm/pkg`
  - Web: `cd web && npm run dev` / `npm test`

### 担当の責務まとめ
| 担当 | 内容 | 主な成果物 |
|---|---|---|
| A | ルール実装 (Rust)、WASM 公開、TS エンジンブリッジ (Worker) | `rules.rs` `wasm.rs` `engine/*.ts` |
| B | AI 実装 (Rust)、強さ検証 | `ai.rs` `selfplay.rs` |
| C | ゲーム制御と UI (TS) | `game/*` `ui/*` `main.ts` `style.css` |

---

## 4. Rust インターフェース

### 4.1 型 (`engine/src/types.rs`、リード)

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side { Sente, Gote }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PieceKind { Lion, Giraffe, Elephant, Chick, Hen }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandKind { Giraffe, Elephant, Chick }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Piece { pub kind: PieceKind, pub owner: Side }

pub type Square = u8; // 0..=11

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Hand { pub giraffe: u8, pub elephant: u8, pub chick: u8 }

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Hands { pub sente: Hand, pub gote: Hand }

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub board: [Option<Piece>; 12],   // serde では長さ 12 の配列 (null 許容)
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
pub enum WinReason { CaptureLion, Try, NoMoves }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawReason { Repetition }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveError {
    GameOver, InvalidSquare, NoPiece, NotOwnPiece,
    IllegalMovement, OccupiedByOwn, NoPieceInHand, SquareOccupied,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AiResult { pub r#move: Move, pub score: i32, pub depth: u32, pub nodes: u32 }

#[derive(Clone, Copy, Debug)]
pub struct AiOptions { pub level: u8, pub max_nodes: u32, pub seed: u32 }
```

`Option<Piece>` の `null` と `Move` の `type` タグ、`AiResult.move` は §2 の JSON と一致させる（`r#move` は serde で `"move"` になる）。

### 4.2 ルール (`engine/src/rules.rs`、担当 A)

```rust
/// 初期局面。history = [初期局面の positionKey]。
pub fn initial_state() -> GameState;

/// 局面のキー (§2.4)。
pub fn position_key(state: &GameState) -> String;

/// 合法手を §2.3 の順で返す。ゲーム終了済みなら空 Vec。
pub fn legal_moves(state: &GameState) -> Vec<Move>;

/// 手を適用して新しい状態を返す。成り・持ち駒の更新・手番交代・history への追記を行う。
/// 不正な手は §2.5 の MoveError を返す（状態は変更しない）。
pub fn apply_move(state: &GameState, mv: &Move) -> Result<GameState, MoveError>;

/// 勝敗判定 (§1.4 の順序に従う)。
pub fn game_status(state: &GameState) -> GameStatus;
```

`MoveError` の割り当て:
- `GameOver`: `game_status != Playing`
- `InvalidSquare`: `from`/`to` が 12 以上
- `NoPiece`: `from` が空 / `NotOwnPiece`: `from` が相手の駒
- `IllegalMovement`: 駒の動きに合わない / `OccupiedByOwn`: 行き先に自駒
- `NoPieceInHand`: 持ち駒がない / `SquareOccupied`: 打つマスが空きでない

優先順は上から。

### 4.3 AI (`engine/src/ai.rs`、担当 B)

```rust
/// AI の指し手を選ぶ。合法手がなければ None。
/// 同じ (state, opts) には同じ結果を返す（seed 以外の乱数・時刻は使わない）。
pub fn choose_move(state: &GameState, opts: &AiOptions) -> Option<AiResult>;
```

**WASM 上では `std::time::Instant` が使えないため、時間制限ではなくノード数で打ち切る。**

| level | 強さ | 内容 |
|---|---|---|
| 1 | よわい | 合法手からランダム（`seed` 使用）。ただし**即勝ちの手があれば必ず指す** |
| 2 | ふつう | 固定深さ 3 の alpha-beta、駒割り＋位置評価。同点手は `seed` で選ぶ |
| 3 | つよい | 反復深化 alpha-beta（最大深さ 12）。`max_nodes` に達したら直前の完了深さの最善手を返す。`max_nodes == 0` なら既定値 `1_000_000` |

全 level 共通の必須要件:
- 即勝ち（ライオン捕獲・トライ成立）の手があれば指す。
- 勝敗は `±10000` 付近のスコア（手数が少ない勝ちを優先）。
- 千日手判定は探索中は無視してよい（`history` は読まなくてよい）。
- 返す手は必ず `legal_moves` に含まれる。`rules.rs` と挙動が食い違わないよう、独自の高速盤面を持つ場合も `rules` と照合するテストを書く。

### 4.4 WASM 公開関数 (`engine/src/wasm.rs`、担当 A)

`wasm-bindgen` で公開する。引数・戻り値はすべて JSON 文字列で、戻り値は `Result` 封筒 `{"ok":true,"value":...}` または `{"ok":false,"error":"..."}`。JSON のパース失敗は `ok:false` で `"invalidJson"` を返し、**panic しない**。

| JS 名 | Rust シグネチャ | 戻り値の `value` |
|---|---|---|
| `initialState` | `fn initial_state_js() -> String` | `GameState` |
| `legalMoves` | `fn legal_moves_js(state_json: &str) -> String` | `Move[]` |
| `applyMove` | `fn apply_move_js(state_json: &str, move_json: &str) -> String` | `GameState`（失敗時 `error` は `MoveError` 文字列） |
| `gameStatus` | `fn game_status_js(state_json: &str) -> String` | `GameStatus` |
| `chooseMove` | `fn choose_move_js(state_json: &str, level: u8, max_nodes: u32, seed: u32) -> String` | `AiResult`（合法手なしは `error: "noMoves"`） |

各関数に `#[wasm_bindgen(js_name = camelCaseName)]` を付ける。`chooseMove` は担当 B の `ai::choose_move` を呼ぶだけの薄いラッパーとする。

---

## 5. TypeScript インターフェース

### 5.1 エンジンクライアント (`web/src/types.ts` に型、実装は担当 A の `web/src/engine/client.ts`)

```ts
export interface AiOptions { maxNodes?: number; seed?: number; }   // 省略時 0 / 乱数

export interface EngineClient {
  initialState(): Promise<GameState>;
  legalMoves(state: GameState): Promise<Move[]>;
  applyMove(state: GameState, move: Move): Promise<Result<GameState, MoveError>>;
  gameStatus(state: GameState): Promise<GameStatus>;
  chooseMove(state: GameState, level: 1 | 2 | 3, opts?: AiOptions): Promise<AiResult>;
  dispose(): void;
}
```

`client.ts` が公開する関数:

```ts
/** Worker を起動し、WASM 初期化の完了後に解決する。 */
export function createEngineClient(): Promise<EngineClient>;
```

- WASM は `worker.ts` 内でロードして実行する（AI 探索で UI を固めないため）。メインスレッドとは `{id, method, args}` / `{id, result | error}` のメッセージで通信する。
- 内部で JSON のシリアライズ/パースを行い、`ok:false` は `Promise` の reject（`Error`、`message` は `error` 文字列）にする。ただし `applyMove` は `Result` をそのまま返す。
- `chooseMove` が `noMoves` のときも reject。

### 5.2 ゲームコントローラ (`web/src/game/controller.ts`、担当 C)

UI と盤面進行の仲介。UI は `GameView` を購読して描画し、入力は下のメソッドへ渡す。

```ts
export type Level = 1 | 2 | 3;

export interface NewGameOptions {
  humanSide: Side;          // 人間の手番側
  level: Level;
}

/** 「選択中」の状態: 盤上の駒か、持ち駒か、なし */
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
  /** 選択中の駒が指せる行き先マス（ハイライト用）。選択なしなら空 */
  targets: Square[];
  lastMove: Move | null;
  /** CPU が考え中か */
  thinking: boolean;
  /** 人間が undo できるか */
  canUndo: boolean;
}

export interface GameController {
  getView(): GameView;
  /** 変更のたびに呼ばれる。戻り値は購読解除関数 */
  subscribe(listener: (view: GameView) => void): () => void;

  /** 新規対局。後手を選んだ場合は CPU の初手まで自動で指す */
  newGame(options: NewGameOptions): Promise<void>;
  /** 盤のマスをクリック。選択・移動・選択変更・解除を状況で判断する */
  clickSquare(square: Square): Promise<void>;
  /** 持ち駒をクリック（人間の手番のときのみ有効） */
  clickHand(kind: HandKind): void;
  clearSelection(): void;
  /** 人間の直前の手と CPU の応手を巻き戻す。CPU 思考中・終局直後は不可 */
  undo(): void;
}

export function createGameController(engine: EngineClient): GameController;
```

クリックの挙動:
- 人間の手番でないとき・`thinking` 中・終局後は入力を無視する。
- 自駒をクリック → 選択。選択中に別の自駒 → 選択を切り替え。同じ駒 → 解除。
- 選択中に `targets` 内のマスをクリック → 手を実行（`move` / `drop`）。実行後、`applyMove` → `gameStatus` を更新し、終局でなければ CPU の手番を `chooseMove` で指す（`thinking: true` の間は入力無視）。
- `targets` は `legalMoves` の結果から選択中の駒の `to` を抽出して求める。
- CPU の手は、人間が見て分かるよう最低 400ms 程度の間を置いて反映する。
- CPU の `seed` はゲームごとに乱数で決める。

### 5.3 UI (`web/src/ui/*`, `main.ts`、担当 C)

関数名は自由（コントローラ以外は公開契約なし）。次の画面要素を実装する:
- 3×4 の盤。後手の駒は 180° 回転して表示。駒は絵文字または SVG/CSS で、ライオン・キリン・ゾウ・ヒヨコ・ニワトリが区別できること。
- 持ち駒エリア（先手・後手それぞれ、種類ごとの枚数表示）。選択中は強調。
- 選択マスと `targets` のハイライト、直前の手（`lastMove`）の強調。
- 手番表示、CPU 思考中の表示、**終局ダイアログ**（勝者・理由・もう一度遊ぶ）。
- 新規対局の設定（先手/後手、レベル 1〜3）、やり直し(undo)ボタン。
- スマホ幅でも盤全体が収まること（タップ操作）。
- `main.ts`: `createEngineClient()` → `createGameController()` → 描画のマウント。

---

## 6. テスト方針

### 6.1 共通テストベクタ
初期局面（先手番）の `legalMoves` は次の **4 手**:
```
move 7→4（ヒヨコが相手のヒヨコを取る）, move 10→6, move 10→8, move 11→8
```
順序は §2.3 のとおり。

### 6.2 担当 A
- 駒種ごとの動きと盤端、移動での成り（先手/後手）、成り駒を取るとヒヨコ、打ち（最奥段含む、成らない）、キャッチ・トライの勝敗判定（トライは取られる/取られない両方）、千日手、`MoveError` 各種。
- ランダム対局を数百局回し、`legalMoves` の全手が `applyMove` で成功し、`positionKey` が一致すること。
- TS: `client.test.ts` では Worker/WASM をモックしてメッセージ処理を検証する。

### 6.3 担当 B
- 全 level で返す手が `legalMoves` に含まれる。
- 即勝ち（ライオンを取れる、トライ成立）を逃さない／相手のライオンを取れる局面で取る。
- 同じ入力・同じ `seed` で同じ結果（決定性）。`max_nodes` を守る（超過は小幅まで）。
- `examples/selfplay.rs`: level 同士の総当たり。強い level が弱い level に勝ち越すことを確認して結果を報告する。

### 6.4 担当 C
- コントローラは `mockEngine` を使った Vitest で、選択 → 移動 → CPU 応手 → 終局、undo、後手選択時の CPU 初手を検証する。

---

## 7. 完了条件
- `cargo test` と `npm test` がすべて通る。
- `wasm-pack build` 後に `npm run dev` でブラウザから最後まで対局できる（先手/後手・3 レベル）。
- 各担当は、自分の担当外のファイルを変更していないこと（`git diff --stat` で確認）。
