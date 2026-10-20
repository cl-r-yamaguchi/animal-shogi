---
name: run-animal-shogi
description: どうぶつしょうぎ(animal-shogi)のWebアプリを実際に起動してブラウザで対局を動かし、スクリーンショットで確認する。アプリを動かす・画面確認・通し対局・E2Eの動作確認を頼まれたときに使う。
---

# どうぶつしょうぎを起動して動かす

テストではなく、実アプリ(WASMエンジン+AI+UI)をブラウザで動かして確認する手順。

## 前提
- Rust(`rustup` の stable + `wasm32-unknown-unknown`)、`wasm-pack`、node、Google Chrome が必要。
- `cargo` が PATH にない場合: `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`

## 手順
1. WASM をビルドする（`web/src/wasm/pkg/` は git 管理外。**これを先にやらないと `npm run build` / dev が失敗する**）
   ```bash
   cd engine && wasm-pack build --target web --out-dir ../web/src/wasm/pkg
   ```
2. 開発サーバを起動して待つ（`timeout` は macOS に無いのでポーリングする）
   ```bash
   cd web && npm install && (npm run dev -- --port 5173 >/tmp/animal-shogi-dev.log 2>&1 &)
   for i in $(seq 60); do curl -sf http://localhost:5173 >/dev/null && break; sleep 1; done   # 起動しなければ /tmp/animal-shogi-dev.log を見る
   # ポート使用中なら先に `lsof -ti:5173 -sTCP:LISTEN | xargs -r kill`
   ```
3. 駆動スクリプトで通し対局し、スクリーンショットを見る
   ```bash
   D=$(mktemp -d) && (cd "$D" && npm init -y >/dev/null && npm i playwright-core >/dev/null)
   NODE_PATH="$D/node_modules" node .claude/skills/run-animal-shogi/drive.cjs --side sente --level 3 --out "$D/shots"
   ```
   - `--side sente|gote`（人間側）、`--level 1|2|3`、`--out` はスクリーンショットの保存先、`--url` で URL 変更可。
   - Chrome が無い環境では `chromium.launch` が失敗する（Google Chrome が必要）。1〜3 は同じシェルで実行する（`$D` を使うため）。
   - 人間側はランダムに指す。終局ダイアログが出れば成功。JS エラー/4xx があれば `errors` に出力され、終了コード 1。
   - 出力された `*.png` は必ず Read で目視する（空白画面は起動失敗）。
4. 終わったら停止する: `lsof -ti:5173 -sTCP:LISTEN | xargs -r kill`

## 注意
- UI 確認は `?mock=1` を付けるとモックエンジンで動く（WASM 不要。ルール/AIの確認には使えない）。
- 盤のセルは `.board .cell`（DOM順 = 画面順）、行き先は `.cell.target`。人間が後手のとき盤は 180° 回転して表示される。
- 駆動スクリプトが確認するのは移動と終局まで。持ち駒打ち・千日手・トライは未カバー（ユニットテストで確認）。
