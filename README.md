# Fractalium

インタラクティブ・フラクタル生成ツール。基本図形と複製ルールを対話的に編集することで、自己相似フラクタルをリアルタイムに生成・閲覧できる。

https://github.com/user-attachments/assets/72efb82c-006f-4da5-bf7b-f2ffbdc30293

## 必要環境

- Rust toolchain（edition 2024 対応版、`rustup` 推奨）

## 起動

```bash
cargo run
```

## 使い方

ウィンドウは左から「Edit キャンバス」「Result キャンバス」「Parameters パネル」の 3 領域に分かれている。

### 1. 基本図形を描く（Edit キャンバス）

Edit キャンバス上でマウス左ボタンをドラッグすると直線を引ける。複数本引くことができる。

| 操作 | 効果 |
|------|------|
| ドラッグ | 自由な直線を引く |
| **Ctrl** + ドラッグ | グリッドスナップ（直交 8 等分・等角 6 等分）。四角形・正三角形が正確に描ける |
| **Shift** + ドラッグ | 45° 単位の角度スナップ |
| **Cmd+Z** | 直前の操作を元に戻す |

- 引いた線を全て消したい場合は Parameters パネルの **Clear lines** ボタンを押す

### 2. 複製ルールを設定する（Parameters パネル）

**+ Add replica** ボタンで複製を追加し、以下のパラメータを編集する:

| パラメータ | 意味 |
|-----------|------|
| TX / TY | 複製の平行移動（[-2, 2] の正規化座標） |
| Rot (deg) | 複製の回転角度（度） |
| Scale | 複製の拡大縮小（0.05〜2.0） |

複製は何個でも追加でき、不要なものは **Delete this replica** で削除できる。

### 3. フラクタルを確認する（Result キャンバス）

Edit と Parameters の変更はリアルタイムに Result キャンバスへ反映される。**Depth** スライダで再帰の深さ（1〜12）を調整する。

Result キャンバス上でマウスホイールを回すと拡大・縮小できる。

> **注意**: 複製数が多い状態で深さを上げると描画負荷が急増する（複製数 R、深さ N のとき R^(N-1) 本の線を描画）。複製 4 つの場合は depth 9〜10 程度を目安にする。

### フラクタル例: 樹木（Y 字フラクタル）

1. Edit キャンバスに縦線を 1 本引く
2. 複製を 2 つ追加し、以下に設定:
   - Replica 0: TX=-0.2, TY=0.3, Rot=25, Scale=0.55
   - Replica 1: TX=0.2, TY=0.3, Rot=-25, Scale=0.55
3. Depth を 6〜8 に上げると樹木状のフラクタルが現れる

## 計画書

開発計画は [`planning/overall_plan.md`](planning/overall_plan.md) を参照。MVP の進行状況は [`planning/mvp/overall_plan.md`](planning/mvp/overall_plan.md) を参照。
