# `app/session`

## 責務

Fractalium アプリ全体、およびユーザーが見る **複数の画面ペイン（種・配置・結果など）や egui による矩形分割** にまたがる **セッション状態** を一元管理する。「どこから入力しても同じワークスペース」となるよう、[フラクタル定義本体](fractal_state.rs)、[編集／Undo／プレースメント](edit_state.rs)、[レイアウトのキャッシュ](layout_cache.rs)、[ビューの一回限りの要求](view_request.rs)、[短命の入力フラグ](interaction_flags.rs) など **`Resource` に分けたモデル**で持ち、画面やプラグインごとに状態をばらけさせない。
