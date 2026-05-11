# Phase 5 — 直下モジュールの再設計と配置（責務起点）

[repository_refactor](overall_plan.md) の延長。現行の `src/*.rs` 一覧やファイル境界は **増築の産物**であり、本 Phase の主眼は **それらをどこへ置くか**ではなく、**正しい責務分割を先に定義し、その分割へコードを吸収する**こととする。移行は縦スライスでよい。**具体的なファイルパスと責務の対応は、下文の「現状」「移行先」「目標ツリー」「旧→新サマリ」に集約する。**

## 目的

- **ユーザー能力**（Phase 1）と **依存の向き**（Phase 3）に整合する粒度で、モジュール境界を **再定義**する。
- 直下に残った「太い」単位（旧 `edit`／`placement`／`fractal`／`state`／…）を **分解しうる境界**を明示し、**ファイル移動だけで終わらせない**。
- 成果物は **`main.rs` 以外がサブツリーに収まる**ことに加え、**各サブツリーの責務が一文で説明できる**こと。

## 現状コードについての前提

- 直下 `*.rs` は **参照設計ではない**。配置の一次資料にしすぎない。
- 再設計で想定する分割は **単一クレート内モジュール**を既定とする（**Cargo ワークスペース化**による複数クレートは本 Phase のスコープ外）。

---

## 現状：`src` ファイルと責務（リポジトリ現状の目安）

**エントリ・配線**

| ファイル | 責務（一文） |
|----------|----------------|
| [main.rs](../../src/main.rs) | クレートの `mod` 宣言のみ。起動は `bootstrap::run` に委譲。 |
| [bootstrap/mod.rs](../../src/bootstrap/mod.rs) | `DefaultPlugins`・各種 `Plugin` の登録順、初期 `Resource`、起動 `Startup`、`FRACTALIUM_BOOT_PRESET` 読みと `FractalState` 初期化、レンダーレイヤ定数。 |

**コア・符号化・プリセット（本 Phase では主に配置の参照点）**

| ファイル | 責務（一文） |
|----------|----------------|
| [core/*.rs](../../src/core/mod.rs) | Bevy 非依存の形状・予算・格子スナップ・種プリセットデータ。 |
| [encoding/*](../../src/encoding/mod.rs) | 共有クエリの平坦なキー／値表現（`flat_query_codec`）。 |
| [fractal_presets/*](../../src/fractal_presets/mod.rs) | 有名フラクタルの静的定義と一覧。 |

**ポート・プラットフォーム**

| ファイル | 責務（一文） |
|----------|----------------|
| [ports/mod.rs](../../src/ports/mod.rs) | I/O 抽象の入口。 |
| [ports/share_link.rs](../../src/ports/share_link.rs) | 共有 URL／履歴まわりのポート宣言。 |
| [ports/png_export.rs](../../src/ports/png_export.rs) | PNG 書き出し・非同期メッセージ用ポート。 |
| [platform/mod.rs](../../src/platform/mod.rs) | 上記ポートのネイティブ／WASM 具象とファクトリ。 |
| [platform/share_nav_*.rs](../../src/platform/mod.rs) | 共有ナビ実装（ターゲット別）。 |
| [platform/png_sink_*.rs](../../src/platform/mod.rs) | PNG シンク実装（ターゲット別）。 |

**アプリケーション（既に `app/`）**

| ファイル | 責務（一文） |
|----------|----------------|
| [app/fractal_share.rs](../../src/app/fractal_share.rs) | リンク用 `#v=…` クエリ本文の **ドメイン意味・検証・`FractalState` との変換**（構文自体は `encoding`）。**モジュール名は誤解を招くため Phase 5 で `share::payload` 等へ改称**（移行先表）。 |
| [app/workspace.rs](../../src/app/workspace.rs) | 再帰予算に基づく深さクランプ、プリセット適用時の `FractalState` 載せ替え。 |

**直下モジュール（Phase 5 で主に分解・移動）**

| ファイル | 責務（一文） |
|----------|----------------|
| [state.rs](../../src/state.rs) | `FractalState`・レイアウト矩形・Undo・Placement 編集中・タッチフラグ・Result カメラフィット要求など **セッション関連 Resource を一括**（分解対象）。 |
| [share.rs](../../src/share.rs) | URL フラグメント同期、`from=share` query 整形、WASM 復元／メッシュ後のアドレス同期、`SharePlugin`。 |
| [platform_handles.rs](../../src/platform_handles.rs) | `ShareNavigation` と `ResultImageOutlet` を束ねる **`Resource` 型**（注入の箱）。 |
| [result_export.rs](../../src/result_export.rs) | Result のオフスクリーン PNG・スクリーンショット・共有メニュー連動・トースト連携の **`ResultExportPlugin`**。 |
| [toast.rs](../../src/toast.rs) | egui トーストのキューと描画、`DeferredToast`。 |
| [grid.rs](../../src/grid.rs) | 格子ガイド描画と、`core::grid_snap` へのスナップ委譲。 |
| [edit.rs](../../src/edit.rs) | Seed キャンバスの入力・ギズモ・選択／Undo キー・**`EditPlugin`**。 |
| [placement.rs](../../src/placement.rs) | Placement キャンバスの操作・ゴースト表示・**`PlacementPlugin`**。 |
| [fractal.rs](../../src/fractal.rs) | Result 側の再帰メッシュ更新・描画・深度クランプシステム・**`FractalPlugin`**。 |
| [view.rs](../../src/view.rs) | Edit／Placement／Result 各カメラのパン・ズーム・タッチ、`fit_result_camera_if_requested`。 |

**プレゼンテーション（既に `ui/`）**

| ファイル | 責務（一文） |
|----------|----------------|
| [ui/mod.rs](../../src/ui/mod.rs) | **`UiPlugin`**；egui メインパスでパネル・ビューポート・トースト・カメラフィット呼び出し。 |
| [ui/layout.rs](../../src/ui/layout.rs) | ワイド／ナローレイアウト、矩形割当。 |
| [ui/shell.rs](../../src/ui/shell.rs) | タイトル・クローム。 |
| [ui/params.rs](../../src/ui/params.rs) | パラメータパネル。 |
| [ui/global_bar.rs](../../src/ui/global_bar.rs) | グローバル操作バー。 |
| [ui/seed_header.rs](../../src/ui/seed_header.rs) | Seed ヘッダ UI。 |
| [ui/depth_controller.rs](../../src/ui/depth_controller.rs) | Result 上 Depth／generations オーバーレイ。 |
| [ui/viewport.rs](../../src/ui/viewport.rs) | egui 矩形から各 `Camera` の `Viewport` を更新（**ワールドの `view.rs` とは別**）。 |

---

## 移行先：ファイル案と責務（必達の名寄せ）

以下は **実装時の置き場の提案**。型名の分割は縦スライスで段階的に行う。

### `app/`（セッション・ユースケース）

| 移行先ファイル（案） | 責務（一文） | 主な現行由来 |
|----------------------|--------------|--------------|
| `app/session/fractal_state.rs` | **`FractalState` だけ**を載せる Resource。保存・共有の芯。 | [state.rs](../../src/state.rs) から分離 |
| `app/session/edit_state.rs` | **Undo／Placement の編集中**（`UndoStack`・`PlacementState`・`PlacementDrag`・キー削除以外の編集境界）。 | [state.rs](../../src/state.rs) |
| `app/session/layout_cache.rs` | **画面上の矩形**（`CanvasLayout`・`UiLayout`・必要なら `ScreenRect`）。UI と同期したヒットテスト用。 | [state.rs](../../src/state.rs) |
| `app/session/view_request.rs` | **一度で消化するビュー意図**（`PendingResultCameraFit`、今後増える同種）。 | [state.rs](../../src/state.rs) |
| `app/session/interaction_flags.rs`（任意） | **`DoubleTapZoomActive`** のような短寿命フラグ（増えないなら `edit_state` に入れてもよい）。 | [state.rs](../../src/state.rs) |
| `app/session/mod.rs` | 上記サブモジュールの公開と、`pub use` の方針整理（過剰な再エクスポートは避ける）。 | 新規 |
| `app/share/payload.rs`（[現 fractal_share.rs](../../src/app/fractal_share.rs)、**モジュール名は `payload` に改称**） | リンク用クエリ本文の **ドメイン意味・検証・`FractalState` との変換**。I/O なし。 | `app/fractal_share.rs` を移し **旧名を廃止** |
| `app/share/sync.rs`（または `app/share/mod.rs`） | URL 同期・WASM 復元・`SharePlugin`、`href_with_from_share` 等。**オーケストレーション**。 | [share.rs](../../src/share.rs) |
| `app/export/mod.rs` | PNG オフスクリーン・`ResultExportPlugin`・`PreparedResultImage`・ビジー状態。 | [result_export.rs](../../src/result_export.rs) |
| `app/platform_handles.rs` | **`PlatformHandles` Resource**（`ports` 実装の束ね）。ロジックは持たない。 | [platform_handles.rs](../../src/platform_handles.rs) |
| `app/session_rules.rs`（改名候補） | 現 [app/workspace.rs](../../src/app/workspace.rs)：**深さクランプ・プリセット適用**。**ファイル名を `layout_cache` と紛らわしくしない**。 | 現 `app/workspace.rs` → リネームし、最終的に `app/session/` へ吸収してもよい |

**既存ファイルの扱い**：`app/workspace.rs` とディレクトリ `app/session/` は **モジュール名が異なるため Rust 上は同居可能**。実装では (a) 既存ファイルの中身を `app/session/preset_apply.rs` 等へ移し `app/workspace.rs` を削除する、(b) 当面 `app/session_rules.rs` にリネームしてから `app/session/` を育てる、のいずれかを選ぶ。

### `ui/canvas/`（ワールド＋共通ガイド）

| 移行先ファイル（案） | 責務（一文） | 主な現行由来 |
|----------------------|--------------|--------------|
| `ui/canvas/seed/mod.rs` | Seed キャンバス入力・ギズモ・**`EditPlugin`**。 | [edit.rs](../../src/edit.rs) |
| `ui/canvas/placement/mod.rs` | Placement キャンバス・**`PlacementPlugin`**。 | [placement.rs](../../src/placement.rs) |
| `ui/canvas/result/scene.rs` | Result の **メッシュ再帰・線分描画・深度クランプ連動**。 | [fractal.rs](../../src/fractal.rs) の主塊 |
| `ui/canvas/result/navigation.rs` | Result／他キャンバス共通でもよい **パン・ズーム・タッチ**（`fit_result_camera_if_requested` 含む）。 | [view.rs](../../src/view.rs) |
| `ui/canvas/result/mod.rs` | `scene` と `navigation` の **`FractalPlugin` 登録**を束ねる場合の親（分割方針に合わせて省略可）。 | 新規 |
| `ui/canvas/grid_overlay.rs` | 格子ガイド描画と `core` へのスナップ委譲。 | [grid.rs](../../src/grid.rs) |
| `ui/feedback/toast.rs`（または `ui/toast.rs`） | egui トースト表示。 | [toast.rs](../../src/toast.rs) |

### `ui/frame/`（既存 `ui/` の収斂：キャンバス本体以外の見た目・パネル）

| 移行先ファイル（案） | 責務（一文） | 主な現行由来 |
|----------------------|--------------|--------------|
| `ui/frame/layout.rs` | ワイド／ナロー・矩形。 | [ui/layout.rs](../../src/ui/layout.rs) |
| `ui/frame/params.rs` | パラメータパネル。 | [ui/params.rs](../../src/ui/params.rs) |
| `ui/frame/global_bar.rs` | グローバルバー。 | [ui/global_bar.rs](../../src/ui/global_bar.rs) |
| `ui/frame/chrome.rs`（任意名） | タイトル・クローム。 | [ui/shell.rs](../../src/ui/shell.rs) 相当 |
| `ui/frame/seed_header.rs` | Seed ヘッダ。 | [ui/seed_header.rs](../../src/ui/seed_header.rs) |
| `ui/frame/depth_controller.rs` | Depth／generations オーバーレイ。 | [ui/depth_controller.rs](../../src/ui/depth_controller.rs) |
| `ui/mod.rs` | **`UiPlugin`** だけを薄くまとめる。 | [ui/mod.rs](../../src/ui/mod.rs) |

### `ui/viewport_bridge.rs`（`frame` と分離：`frame` と同種の類型ではなく **レイアウト結果→カメラ** の同期）

| 移行先ファイル（案） | 責務（一文） | 主な現行由来 |
|----------------------|--------------|--------------|
| `ui/viewport_bridge.rs` | **egui が割り当てた矩形から各 `Camera` の `Viewport` を更新**（ワールドの `view.rs` とは別）。パネル描画ではなく描画パイプライン側への適用。**モジュール名はワールド `view` と衝突させない**。 | [ui/viewport.rs](../../src/ui/viewport.rs) |

### エントリ

| ファイル | 変更内容（案） |
|----------|----------------|
| [main.rs](../../src/main.rs) | `mod edit;` 等を削減し、`mod app;` `mod ui;` 配下の宣言に置き換える。 |

---

## 再設計：責務レイヤ（先に固定）

下から上へ。Phase 3 の `core` / `ports` / `platform` / `bootstrap` の役割は維持する。

### 1. コア領域（既存 `core` と同一思想）

- **フラクタルの意味論**：基図形、IFS、予算・深さ上限の **純データと純関数**。
- **リンク用クエリの構文**（クエリキーと値の形）は `encoding` が担い、**ドメイン検証・セッション状態への適用規則**はアプリ寄りでよい（現状の分離を維持・明確化）。

### 2. セッション分解（`app` 内の再編の核）

いま `state.rs` 等に混在しがちなものを **意図で分ける**（実装は一ファイルから複数サブモジュールへ段階的に分割してよい）。

**語彙**：「state」は **フラクタルの真実**と**編集中の一時情報**にだけ使い、それ以外は **キャッシュ**・**未処理の要求** など別名にする（すべてを `*State` と呼ぶと誤解が増える）。

| 名前（目安） | 説明 |
|--------------|------|
| **fractal_state** | 保存・共有の対象になる **フラクタル定義**（基図形・レプリカ・深さ・表示フラグ）。実装上は既存の `FractalState` に相当する「本命」データ。 |
| **edit_state** | 選択・ドラッグ途中・Placement の一時操作、Undo／Redo スタック。**確定前**の編集情報と、確定 **単位の規則**（何を一度の編集とみなすか）。 |
| **layout（キャッシュ）** | 画面上の矩形（どこが Seed／Result か等）。**fractal_state から導かれる不変ではなく**、フレーム（レイアウト UI）と同期した **UI 幾何の写し**。ヒットテストに必要。`state` より **キャッシュ／スナップショット** のニュアンス。 |
| **ビュー要求（例：カメラフィット）** | 「次にカメラをフィットさせたい」など **一度処理したら消える意図**。フラクタル定義でも編集途上でもなく、**未処理キュー／リクエスト**として扱う。 |

**原則**：型を一つの巨大 `Resource` に載せ続けない。境界が濁ったら **fractal_state / edit_state** のように **別型・別 mod** で切る。

### 3. ユースケース編成（`app`）

- **プリセット適用**、**Undo 境界**、**共有の符号化／復号の手順と検証**、**深度クリップと予算**、**エクスポートの成否種別**など、Phase 2 の「アプリケーション」に相当する **ルールとシーケンス**。
- **URL 同期・起動時復元**の **手順のオーケストレーション**（どのタイミングで符号化し、どのタイミングでポートを呼ぶか）もここ。ブラウザ API やファイルは触らない。

### 4. ポートと束ね（`ports`・`app` の境界）

- **trait とメッセージ形**は `ports`（または Phase 3 の折りたたみどおり `app` 側の宣言で両立）。
- 複数ポートを **1 つの Bevy `Resource` に束ねる**のは **「起動時配線用の薄い箱」**として位置づけ、**ドメインロジックを持たない**（現 `platform_handles` 相当）。

### 5. プレゼンテーション（`ui`）

次の **サブ責務**に分けて考える（ディレクトリ名は実装時に確定）。

| サブ領域 | 責務 |
|----------|------|
| **フレーム** | ワイド／ナロー、パラメータパネル、グローバルバー、タイトル帯。既存 `ui/` の一部。実装の置き場は `ui/frame/`。**ビューポート同期**（egui の矩形→各カメラ `Viewport`）は同じ「見た目のパネル」とは別役割のため `ui/viewport_bridge.rs` に分離（移行先表）。 |
| **結果のオーバーレイ** | Depth・世代トグルなど **egui で Result 上に重ねるコントロール**。 |
| **ワールドキャンバス（Seed）** | ポインタ・キーから **意図**を起こし、ドキュメント更新はアプリ規則へ委譲。ギズモとガイド描画。 |
| **ワールドキャンバス（Placement）** | 同上。レプリカ操作と視覚化。 |
| **ワールドキャンバス（Result）** | ここを **一枚の「fractal.rs」にまとめない**。少なくとも概念的に **(A) シーン表現（メッシュ・再帰同期）** と **(B) カメラのパン・ズーム・タッチ** と **(C) 共有 URL 同期トリガなどへのフック** を分離し、**(A)(B)** は「見せる・動かす」、(C) は **イベント／リソース経由で app に寄せる**ことを検討する。 |
| **格子ガイド** | スナップの数式は `core`、**ドットや補助線の描画**はプレゼンテーション。 |
| **トースト** | 表示と非同期メッセージのフラッシュ。**出す／出さないの判断**は app 規則。 |

### 6. 画像エクスポート（`app` 主導、`ui` は触発と表示）

- **オフスクリーンの要求、ビジー状態、スクリーンショット取得、ポートへのバイト列渡し**はユースケース（`app`）。
- **メニューからの要求・進捗表示**は `ui`。

### 7. `bootstrap`

- プラグイン順序・レンダーレイヤ・起動時に **環境から fractal_state の初期値**を読む場所の **配線**のみ。ドメイン計算を増やさない。

---

## 目標とするツリー（ファイル粒度の目安）

論理ツリーを **想定ファイル名**まで落とした形（実装で空ファイルや名前変更あり）。詳細な責務は上記「移行先」表を正とする。

```
src/
  main.rs
  bootstrap/mod.rs
  core/ …
  encoding/ …
  fractal_presets/ …
  ports/ …
  platform/ …
  app/
    mod.rs
    platform_handles.rs
    session_rules.rs        … 旧 app/workspace.rs（リネーム例。session 配下へ吸収するまでの置き場）
    session/
      mod.rs
      fractal_state.rs
      edit_state.rs
      layout_cache.rs
      view_request.rs
      interaction_flags.rs    … 任意
    share/
      mod.rs
      payload.rs              … リンククエリのドメイン（現 `fractal_share`、改称して吸収）
      sync.rs                 … 現 share.rs（URL 同期・プラグイン）
    export/
      mod.rs
  ui/
    mod.rs
    viewport_bridge.rs      … 現 ui/viewport.rs（egui 矩形 → Camera Viewport。`frame` 外）
    frame/
      layout.rs
      params.rs
      global_bar.rs
      chrome.rs
      seed_header.rs
      depth_controller.rs
    feedback/
      toast.rs                … または ui/toast.rs
    canvas/
      grid_overlay.rs
      seed/mod.rs
      placement/mod.rs
      result/
        mod.rs
        scene.rs
        navigation.rs
```

---

## 旧→新 対応サマリ（直下モジュールのみ）

| 現ファイル | 移行先（主） |
|------------|----------------|
| [state.rs](../../src/state.rs) | `app/session/*.rs`（複数） |
| [share.rs](../../src/share.rs) | `app/share/mod.rs` 等 |
| [platform_handles.rs](../../src/platform_handles.rs) | `app/platform_handles.rs` |
| [result_export.rs](../../src/result_export.rs) | `app/export/mod.rs` |
| [toast.rs](../../src/toast.rs) | `ui/feedback/toast.rs` 等 |
| [grid.rs](../../src/grid.rs) | `ui/canvas/grid_overlay.rs` |
| [edit.rs](../../src/edit.rs) | `ui/canvas/seed/mod.rs` |
| [placement.rs](../../src/placement.rs) | `ui/canvas/placement/mod.rs` |
| [fractal.rs](../../src/fractal.rs) | `ui/canvas/result/scene.rs` 中心 |
| [view.rs](../../src/view.rs) | `ui/canvas/result/navigation.rs` |

**意図**：`result` の **scene** と **navigation** を分け、増築で混ざった **「見た目のシステム」と「入力での移動」** を読み分ける。トリガとポリシーは **app のイベント／リソース**に寄せる。

---

## 計画概要（実施順）

### Task 1 — 境界の合意

- 上記 **セッション分解（fractal_state・edit_state・layout・ビュー要求）**と **Result＝scene＋navigation** の分割を人間レビューで合意する（ディレクトリ名は合意後に微調整可）。

### Task 2 — サブツリー用意と縦スライス

- `app/session/` または `ui/canvas/` の **空の子 mod** から入り、**一つずつ**旧実装を移し、旧パスは短い期間 **ファサード**でもよい。
- **既存 `app/workspace.rs` の移行方針**（`app/session/` へ統合するか、`session_rules.rs` 経由か）は着手前に決める。
- **依存の向き**：[Phase 3 依存表](phase3_layout.md#モジュール間依存ルール許可関係) を毎スライス確認。

### Task 3 — 直下 `*.rs` の消滅

- `main.rs` 以外、直下に単体 `*.rs` が残らないこと。

### Task 4 — 既存 `app/`・`ui/` 内の収斂

- Phase 5 の主対象は元来直下だったファイルだが、`app/share`（`payload`／旧 `fractal_share`）と `app/workspace.rs`／`session_rules` など **名前と責務が重なる**箇所は、同一の **session / share / export** の語彙に **段階的に統合**する（別 PR でもよい）。

---

## 受け入れ条件

- [ ] **責務表**（fractal_state／edit_state／layout／ビュー要求、Result の scene／navigation、export／share／toast）と矛盾する置き場が残る場合は、例外と理由を本文または Issue に明示している。
- [ ] `main.rs` 以外 **`src/*.rs` 単体ファイルが存在しない**。
- [ ] `cargo build` 成功。可能なら **WASM** も CI 同旨で成功。
- [ ] Phase 3 依存表に反する依存（**`ui`→`platform` 直** 等）がない。
- [ ] 新しい `app/session`・`ui/frame`（および `viewport_bridge`）と `ui/canvas` の README 相当の **モジュールドキュメント（`//!`）** が、初見で境界を説明できる。

## 実施記録

- （着手日・マイルストーン・合意したディレクトリ名の確定を追記）
