# Phase 2: ドラッグ確定タイムラグ解消

## 目的

Base Shape の線描画、Placement のレプリカ操作（移動・スケール・回転）、Params の `DragValue` のいずれにおいても、
**マウスを離してから値が確定（線が引かれる／スナップ位置に止まる）するまでに体感タイムラグがある** 問題を解消する。

## 想定される原因

事前調査での仮説（実装着手時に検証する）：

1. **入力読み出しと描画の順序**
   - `handle_drag_input` → `draw_canvas` の `chain()` が `Update` スケジュールに入っているが、`MouseButton` の `just_released` を読むタイミングと描画の順序のズレで、`Idle` への遷移が描画後に反映されている可能性。
2. **egui との優先順位**
   - `EguiPrimaryContextPass` で描画される egui の入力処理（`wants_pointer_input`）と、Bevy の `ButtonInput<MouseButton>` の更新タイミングのズレで、リリースを 1〜2 フレーム取りこぼしている可能性。
3. **カーソル座標の取得タイミング**
   - `just_released` の同フレームで `cursor_in_edit` / `cursor_in_placement` が `None` を返すケース（ビューポート境界・スケーリング）で、確定処理が次フレームへ持ち越されている可能性。
4. **VSync / 入力サンプリング遅延**
   - Bevy 0.18 のデフォルト挙動で、`MouseInput` イベントが次フレームまでバッファされている可能性。`Res<AccumulatedMouseMotion>` 系との混在を疑う。
5. **Params の DragValue**
   - egui の `DragValue` は内部で「離してから値反映」の挙動が変わるバージョンがある。`speed` 設定や `change_on_release` の有無を確認する。

## Tasks

### Task 1: 再現条件の確定

- 各パネル（Base Shape / Placement / Params）で、ドラッグ → リリース → 値確定までの所要を計測。
- `MouseButton::just_released` と「最終的に状態が確定したフレーム」を `info!` ログで記録し、フレーム差分を確認。
- リリース直後に egui がポインタを掴んでいるか（`wants_pointer_input`）の値も同フレームでログ。

### Task 2: 原因切り分け

ログ結果を踏まえ、以下のいずれが主要因か特定：

- A. システム実行順序（`chain()` 内の順序、または `EguiPrimaryContextPass` との順序）
- B. egui の入力ガード（`wants_pointer_input` がリリースフレームで true になり、確定がスキップされている）
- C. 入力イベントのバッファリング（リリース判定が 1 フレーム遅れる）
- D. Params 固有問題（DragValue の挙動）

### Task 3: 修正

特定された原因に応じて最小限の修正を入れる。修正候補例：

- A の場合：システム順序を見直し、`handle_drag_input` を `PreUpdate` または `EguiPrimaryContextPass` の前に移す／チェーンを再構成。
- B の場合：`just_released` の処理だけは egui の `wants_pointer_input` に関わらず実行する（押下時にすでにキャンバスを掴んでいた場合のみ確定する状態フラグを `DrawState` / `PlacementDrag` 側に持たせる）。
- C の場合：`MouseButtonInput` の `EventReader` に切り替え、当該フレームで届いたイベントを直接処理する。
- D の場合：`DragValue` の構成を見直す（`update_while_editing(false)` 等）。

## 設計メモ

- 修正は **タイムラグ解消** のみが目的。動作の意味（スナップ、Undo、egui ガード）は維持する。
- Edit / Placement で同じ修正パターンが必要なら共通ヘルパに括り出す（過度な抽象化はしない）。
- Phase 3（線編集）でも同じ入力ロジックを使うので、ここで安定した入力基盤を整える。

## 受け入れ条件

- [ ] Base Shape で線をドラッグして離した瞬間、目視でほぼ即座に確定線が描かれる（プレビュー線→確定線への切り替えが 1 フレーム以内）
- [ ] Placement のレプリカ移動／スケール／回転で、ドラッグ終了後に位置が止まる（リリース後の追従や逆戻りが起きない）
- [ ] Params の `DragValue` でドラッグして離すと即時にフラクタルが反映される
- [ ] 既存の egui パネル上クリックでキャンバスにドラッグ判定が漏れない挙動は維持
- [ ] Undo の積みタイミングが従来どおり（連続編集中に複数積まれない）
