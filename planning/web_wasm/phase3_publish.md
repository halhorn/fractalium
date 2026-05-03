---
name: Phase 3 - 公開・配布
type: project
---

# 目的

`main` への push をトリガに、GitHub Actions で WASM ビルドを実行し、GitHub Pages 上の公開 URL から起動できる状態をつくる。

# 計画概要

| Task | 概要 |
|------|------|
| T3.1 | GitHub Actions ワークフローを追加（Rust toolchain + trunk セットアップ + ビルド） |
| T3.2 | GitHub Pages へのデプロイ自動化（公式 `actions/deploy-pages` を採用） |
| T3.3 | リポジトリ設定で Pages を有効化し、公開 URL を確定する |
| T3.4 | 公開 URL を README とリポジトリ description に掲示する |

# 計画詳細

## T3.1: ビルドワークフロー

`.github/workflows/deploy.yml` を新規追加。

- トリガ: `push` to `main`、`workflow_dispatch`
- ジョブ構成:
  1. `actions/checkout`
  2. `dtolnay/rust-toolchain@stable` で `wasm32-unknown-unknown` 追加
  3. `Swatinem/rust-cache` で `target/` をキャッシュ
  4. `jetli/trunk-action` または `cargo install --locked trunk` で trunk 取得
  5. `trunk build --release --public-url /fractalium/`
  6. `actions/upload-pages-artifact` で `dist/` をアップロード

ビルド時間目安: 初回 5〜10 分、キャッシュ後 2〜3 分。

## T3.2: GitHub Pages へのデプロイ

ワークフローに deploy ジョブを追加：

- `permissions: pages: write, id-token: write`
- `actions/deploy-pages` を使用
- `concurrency: group: pages, cancel-in-progress: true`

`gh-pages` ブランチ方式ではなく **公式 GitHub Pages Actions** を使う（ブランチを汚さない、設定がシンプル）。

## T3.3: Pages 設定

GitHub リポジトリ設定で：

- Settings → Pages → Source を **GitHub Actions** に変更
- 公開 URL: `https://<user>.github.io/fractalium/`
- カスタムドメインは設定しない（Q&A 確定済み）

## T3.4: 導線整備

- README 冒頭にバッジ風のリンク：「▶ Try it in your browser」
- リポジトリの About 欄（GitHub UI 右側）に公開 URL を設定
- 任意で OGP 用のスクリーンショットを `index.html` の `<meta>` に追加（SNS シェア時の見栄え）

# 受け入れ条件

- [ ] `.github/workflows/deploy.yml` が `main` 上に存在する
- [ ] `main` への push 後、Actions が自動実行され緑（成功）になる
- [ ] `https://<user>.github.io/fractalium/` で Fractalium が起動する
- [ ] 公開 URL がリポジトリの About 欄に設定されている
- [ ] 公開後の URL から、Phase 1 / Phase 2 で確認した機能が一通り動作する
- [ ] PR ブランチでは Pages デプロイが走らない（`main` 限定）
