# どうぶつしょうぎ (animal-shogi)

Rust(WASM) でルール判定とAI、TypeScript(Vite) でUIを実装する人間 vs コンピュータのWebアプリ。

## 規約
- **インターフェース（型・JSON形式・関数シグネチャ・ファイル所有権）は `docs/spec.md` が正。** 変更が必要なら、実装前に spec を更新する相談をする。
- **担当外のファイルは編集しない。** 担当は A: `engine/src/rules.rs`・`wasm.rs`・`web/src/engine/*`、B: `engine/src/ai*`・`examples/selfplay.rs`、C: `web/src/game/*`・`ui/*`・`main.ts`・`style.css`。共有ファイル（`Cargo.toml`・`lib.rs`・`types.rs`・`web/src/types.ts`・Vite設定）はリードのみ（詳細は spec §3）。
- テスト: Rust は `cd engine && cargo test`、WASM ビルドは `cd engine && wasm-pack build --target web --out-dir ../web/src/wasm/pkg`、Web は `cd web && npm test`（開発サーバは `npm run dev`）。
- 実アプリをブラウザで動かして確認するときは `.claude/skills/run-animal-shogi`（`/run-animal-shogi`）の手順に従う。
- 回答・コメント・Issue/PR は日本語で書く。

## 開発フロー
作業は**すべて Issue から始める**。

1. **Issue を作る**: 次の4点を必ず書く。
   - 背景
   - やること
   - 受け入れ条件
   - スコープ外
2. **ブランチ作成**: 「Issue #N に対応するブランチを切って」で `fix/issue-N`（機能追加は `feat/issue-N`）を `main` から切る。
3. **計画**: Plan Mode で対応方針を出し、人が確認して承認してから実装に入る。
4. **実装**: 小さく実装し、都度テストを回す。変更は Issue のスコープ内に収める。
5. **PR**: 終わったら PR を作る。本文に `Closes #N`、受け入れ条件の充足状況、テスト結果を書く。
6. **コードレビュー**: `/code-review` でレビューし、指摘を反映してから**マージ**する。

## チーム共有
- スキル・エージェント・設定は `.claude/` にコミットしてチームで共有する（個人用の `.claude/settings.local.json` はコミットしない）。
- 共有すべきルールや手順が増えたら、この `CLAUDE.md` か `docs/` に文書化する。
