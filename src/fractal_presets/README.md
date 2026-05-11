# `fractal_presets`

## 責務

有名フラクタルなどの **静的定義**と **一覧**。データはコアに近い形でここに置き、**プリセット適用手続きや Undo と境界をどう敷くか**は [`app`](../app/mod.rs)（例: [`session_rules`](../app/session_rules.rs)）側の責務。
