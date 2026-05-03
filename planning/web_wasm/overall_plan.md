---
name: Web (WASM) 配布計画
type: project
---

# 目的

Fractalium をブラウザ上で動作させ、URL アクセスのみでフラクタル生成体験を提供する。インストール不要で Mac / Windows / iOS / Android すべてに到達できるため、ユーザー獲得の費用対効果が最も高い配布形態とする。

# 計画概要

Bevy 0.18 + bevy_egui 0.39 は WebAssembly + WebGL2 を公式サポートしており、現状のソースコードはファイル I/O やスレッドなど WASM 非互換 API を使っていない。したがって作業の中心は「ビルド設定」「ブラウザ向けのウィンドウ／入力／タッチ調整」「配布パイプライン」の 3 点となる。

| Phase | 概要 | 詳細 |
|-------|------|------|
| Phase 1 | WASM ローカルビルドが通り、ブラウザで起動できる | [phase1_build.md](phase1_build.md) |
| Phase 2 | デスクトップ／モバイル両ブラウザでの操作性とパフォーマンスを最適化する | [phase2_browser_mobile.md](phase2_browser_mobile.md) |
| Phase 3 | GitHub Pages へ自動デプロイし、公開 URL から起動できる | [phase3_publish.md](phase3_publish.md) |
| Phase 4 | README とリポジトリ設定を更新し、ユーザーへの導線を整える | [phase4_docs.md](phase4_docs.md) |

# 受け入れ条件

- [ ] Phase 1 の受け入れ条件をすべて満たしている（[phase1_build.md](phase1_build.md)）
- [ ] Phase 2 の受け入れ条件をすべて満たしている（[phase2_browser_mobile.md](phase2_browser_mobile.md)）
- [ ] Phase 3 の受け入れ条件をすべて満たしている（[phase3_publish.md](phase3_publish.md)）
- [ ] Phase 4 の受け入れ条件をすべて満たしている（[phase4_docs.md](phase4_docs.md)）

# 全体方針

## ターゲット環境

- **デスクトップ**: 最新版 Chrome / Edge / Safari / Firefox
- **モバイル**: iOS Safari / Android Chrome（タッチ操作で線描画・ピンチズーム）
- **描画 API**: WebGL2（Bevy 0.18 のデフォルト互換性重視）
- **配布**: GitHub Pages 上の静的ホスティング（`--public-url /fractalium/`）

## ビルドツール

`trunk` を採用する（Bevy Cheatbook 公式推奨、`Trunk.toml` + `index.html` のみで `dist/` が完成し GitHub Pages に直接配信できる）。`wasm-server-runner` はローカル動作確認用に併用してもよい。

## ネイティブ版との関係

ブラウザ版は `cargo run` のネイティブ版と厳密一致させない。WASM 制約に起因する差異（フォント・クリップボード・初期ロード時間等）は許容し、必要なら README に注記する。

## スコープ外

- WebGPU バックエンド（Safari の対応が安定するまで保留。`webgl2` で十分）
- ネイティブアプリ配布（Mac / Windows バイナリ）は別 Feature として扱う
- 作品の保存・共有 URL 機能（既存の Phase 4 = 仕上げ で扱う）

# 確定事項（Q&A 履歴）

- 公開先: `https://<user>.github.io/fractalium/`（独自ドメインなし、`--public-url /fractalium/`）
- スマホ対応: Phase 2 のスコープに含める（タッチ描画・ピンチズーム）
- ネイティブ版との挙動一致: 不要（WASM 制約による差異は注記で許容）
