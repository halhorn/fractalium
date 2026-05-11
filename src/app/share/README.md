# `app/share`

## 責務

共有を **クエリ構文・インフラ I/O とドメインロジックから切り離す**。[`payload`](payload.rs) は `#v=…` 等のクエリ本文の **ドメイン的意味・検証・`FractalState` との変換**（フラット表現自体は [`encoding`](../../encoding/mod.rs)）。[`sync`](sync.rs) は URL フラグメントや WASM 復元などの **手順全体** と `SharePlugin`。
