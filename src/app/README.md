# `app`

## 責務

**ワークスペースのルールと編成**。共有の符号化・復号の流れ、Undo／プリセット適用と深度クランプ、エクスポート駆動、URL 復元などのユースケース。**`trait`** は [`ports`](../ports/mod.rs)、環境側の具象は [`platform`](../platform/mod.rs)。この層は **イベントと `Resource`** を通じて状態を整合させる。

[`ui`](../ui/mod.rs) はここ経由でのみドメインポリシーを触ることを目標にし、`ui` からプラットフォーム具象へ **直接依存しない**。

## 直下・サブツリー（要約）

| サブツリー／ファイル | 責務（一文） |
|----------------------|----------------|
| [`session`](session/mod.rs) | アプリ全体・複数画面ペインにまたがるセッション状態の一元管理（`Resource` で型分割） |
| [`share`](share/mod.rs) | クエリペイロードの検証、`SharePlugin` を含む URL 同期などのオーケストレーション |
| [`export`](export/mod.rs) | 結果 PNG のオフスクリーン・ポート連携、`ResultExportPlugin` |
| [`platform_handles`](platform_handles.rs) | `ports` 実装を束ねる **注入用 `Resource`。ロジックは持たない** |
| [`session_rules`](session_rules.rs) | 再帰予算に沿った深さクランプ、プリセット適用時の状態の載せ替え |
