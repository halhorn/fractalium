# F1: プロジェクト初期化

## 目的

Rust + Bevy のフラクタル生成アプリの開発・実行・検証ができる最小プロジェクト雛形を整える。後続の F2 以降が「中身を実装するだけ」の状態でスタートできるようにする。

## 計画概要

| ID | 内容 |
|----|------|
| T1 | `cargo init` によるクレート初期化（バイナリ crate） |
| T2 | `Cargo.toml` に依存追加（Bevy 0.18 系 + UI クレート） |
| T3 | `.cargo/config.toml` で開発ビルド最適化（galaxy と同方針） |
| T4 | `.gitignore` 配置（Rust + IDE + OS） |
| T5 | `src/main.rs` に空ウィンドウを開くだけの最小 Bevy アプリを実装 |
| T6 | `README.md` に概要・起動方法・操作（現状なし）を記載 |
| T7 | `cargo build` / `cargo run` / `cargo clippy` / `cargo fmt --check` の通過確認 |

## 計画詳細

### T1: cargo init

- 場所: `/Users/halhorn/work/fractalium`
- バイナリ crate（ライブラリは MVP では不要）
- crate 名: `fractalium`
- edition: `2024`（galaxy と同じ）

### T2: 依存追加

確定:
- `bevy = "0.18"` — メインエンジン

[Question]
編集 UI（複製パラメータのスライダ・複製リスト等）に Bevy 標準 UI を使うか、`bevy_egui` を使うか。
[/Question]

[Answer]
`bevy_egui` を採用する。理由:
- 本 PJ の操作部はエディタ風 UI（複製パラメータ編集、複製リストの追加削除、深さスライダ等）であり、即時モード GUI の egui が実装速度・保守性で優位
- フラクタル本体（基本図形・複製の輪郭）は Bevy 標準描画、編集 UI は egui、と役割分担を明確化できる
- Bevy 標準 UI のみで作るとレイアウトとイベント配線のコストが MVP スコープに見合わない
- バージョン互換は T2 着手時に Bevy 0.18 対応版を確認して採用
[/Answer]

### T3: `.cargo/config.toml`

galaxy と同じ方針:

```toml
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

理由: 自分のコードはコンパイル速度重視（opt-level=1）、依存（特に Bevy）は最適化（opt-level=3）で実行時の重さを回避する。

### T4: `.gitignore`

galaxy と同様の内容:

```
/target/
**/*.rs.bk
.idea/
.vscode/
*.swp
*.swo
*~
.DS_Store
Thumbs.db
```

### T5: `src/main.rs` の最小実装

責務:
- Bevy `App` を構築
- `DefaultPlugins` を追加
- ウィンドウタイトル: "Fractalium"
- 背景色を設定（暗めの単色）
- 2D カメラを配置（後続 F3 で正規化座標キャンバスに置き換える）

公開する `pub fn` は無し（`fn main()` のみ）。具体的なシステム関数はこの段階では追加せず、起動確認だけを目的とする。

### T6: README.md

含める項目:
- プロジェクト概要（1-2 段落）
- 必要環境（Rust toolchain）
- 起動方法（`cargo run`）
- 計画書へのリンク（`planning/overall_plan.md`）

### T7: 通過確認コマンド

- `cargo build` が成功
- `cargo run` でウィンドウが起動し、暗背景の空画面が表示される
- `cargo clippy -- -D warnings` で警告ゼロ
- `cargo fmt --check` で差分ゼロ

## 受け入れ条件

- [x] `Cargo.toml` が存在し、`bevy = "0.18"`（および確定した UI クレート）が依存に含まれる
- [x] `.cargo/config.toml` が dev profile の opt-level 設定を持つ
- [x] `.gitignore` が `/target/` を含む
- [x] `src/main.rs` が空ウィンドウを開くだけの最小実装になっている
- [x] `cargo run` でウィンドウタイトル "Fractalium" の暗背景ウィンドウが起動する
- [x] `cargo clippy -- -D warnings` が成功する
- [x] `cargo fmt --check` が差分なしで成功する
- [x] `README.md` に起動方法と計画書へのリンクが記載されている
- [ ] git に初回コミットが入っている（`Cargo.toml`, `Cargo.lock`, `src/`, `.gitignore`, `.cargo/`, `README.md`, `planning/`）

## 範囲外（F1 では実装しない）

- 操作部 / 結果表示部のレイアウト → F2
- [-1, 1] 正規化座標キャンバス → F3
- 状態モデル → F4
- 描画・編集機能 → Features
