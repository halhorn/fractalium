# F4: フラクタル状態モデル

## 目的

フラクタル生成に必要な「基本図形」「複製ルール」「描画深さ」を 1 つの Resource として定義し、後続の Features がこの状態を読み書きするだけでよい状態にする。さらに、状態が UI に伝搬することを検証するため、egui パネルから「深さ」を変更できる最小スライダを実装する。

## 設計方針

- 状態は **単一の Bevy Resource** (`FractalState`) に集約する
- 値型のみで構成し、`Clone`/`Default` を派生する。レンダラやエディタはこの構造体を読み書きするだけ
- 複製の変換は MVP 方針通り「位置・回転・等方スケール」の 3 要素 (`Replica`)
- 基本図形は MVP では「直線の集合」だけを保持 (`BaseShape::lines`)。Phase 2 で図形プリセット用の variant が増える前提で、後で enum 化しやすい命名にする
- F4 単体では、フラクタル本体の再帰計算や描画は **行わない**。状態の存在と UI 伝搬の確認のみ

## 計画概要

| ID | 内容 |
|----|------|
| T1 | `state.rs` モジュールを作成し、`FractalState` / `BaseShape` / `Line` / `Replica` を定義 |
| T2 | `FractalState` を `Resource` として App に登録、初期値（空の base_shape、空の replicas、depth=4）を投入 |
| T3 | `params_panel` から `FractalState` を `ResMut` で参照し、深さスライダ（1〜7）を表示 |
| T4 | 同パネルに `lines.len()` / `replicas.len()` を read-only で表示し、状態が UI に届いていることを目視できるようにする |
| T5 | clippy / fmt / screencapture 通過 |

## 計画詳細

### T1: 型定義

```rust
// state.rs
#[derive(Resource, Default, Clone)]
pub struct FractalState {
    pub base_shape: BaseShape,
    pub replicas: Vec<Replica>,
    pub depth: u32,
}

#[derive(Default, Clone)]
pub struct BaseShape {
    pub lines: Vec<Line>,
}

#[derive(Clone, Copy)]
pub struct Line {
    pub a: Vec2,
    pub b: Vec2,
}

#[derive(Clone, Copy)]
pub struct Replica {
    pub translation: Vec2,
    pub rotation: f32, // radians
    pub scale: f32,    // uniform; > 0
}
```

- 全フィールドは pub。MVP 段階で隠蔽の必要はない
- 後続の Phase で `Replica` を `enum`（Affine / Shear etc.）に拡張する場合は struct → enum でリファクタする
- `Replica` の単位は「親キャンバスの正規化座標 [-1, 1] における値」。これはコード上のコメントに明記

### T2: 初期値

```rust
fn default_state() -> FractalState {
    FractalState {
        base_shape: BaseShape::default(),
        replicas: vec![],
        depth: 4,
    }
}
```

`App::insert_resource(default_state())`。MVP 開始時点では描画対象が空なので、F3 で見た原点マーカと枠だけが見える状態は維持される。

### T3: 深さスライダ

`params_panel` に以下を追加:

```rust
ui.heading("Parameters");
ui.separator();
ui.add(egui::Slider::new(&mut state.depth, 1..=7).text("Depth"));
ui.label(format!("Lines: {}", state.base_shape.lines.len()));
ui.label(format!("Replicas: {}", state.replicas.len()));
```

`state` は `mut state: ResMut<FractalState>` で受け取る。

### T4: 範囲

- 深さ: 1〜7。8 以上は MVP では負荷が読めないので除外（Phase 4 で見直し）
- 上限拡張は後続 Phase

## 受け入れ条件

- [x] `state.rs` (もしくは `src/state.rs`) に `FractalState` / `BaseShape` / `Line` / `Replica` が定義されている
- [x] `FractalState` が `Resource` として App に登録され、起動時に depth=4・空の base_shape・空の replicas で初期化される
- [x] params パネルに "Depth" のスライダが表示され、値を変えると同パネル内の表示も更新される
- [x] params パネルに "Lines: 0" / "Replicas: 0" が表示される
- [x] `cargo clippy -- -D warnings` が通る
- [x] `cargo fmt --check` が差分なしで通る
- [x] screencapture で自走確認 OK（depth スライダが見える）

## 範囲外

- 直線の追加 UI → Feat1
- 複製の追加・編集 UI → Feat2
- フラクタル再帰描画 → Feat3
- リアルタイム反映の体感確認 → Feat4
- 状態の保存・読込 → Phase 4
