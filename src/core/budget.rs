//! 再帰フラクタルの**演算予算**から許容できる `depth` を求める。
//!
//! 深さと複製数がどう並ぶかで、再帰の訪問回数や線分の本数は指数的に伸びうる。そのまま広い上限を許すと、とくにブラウザ環境では
//! スタック不足や応答停止、メッシュ肥大化によるメモリ圧迫といった問題を招きやすく、WASM ではより重くのしかかりやすい。
//! ほかにも、共有 URL や UI が許す **`depth` の絶対上限**をコードの一箇所に置いておけば、入力の検証と
//! 実際に動かす再帰とが食い違わなくなる。**`depth` の絶対上限**と、訪問ノード数・線分本数の目安上限がどう組み合わさるかは、
//! [`FRACTAL_DEPTH_HARD_CAP`] のドキュメント先頭に書く。

/// ## 上限 depth 決定のアルゴリズム概要
///
/// [`FRACTAL_DEPTH_HARD_CAP`]・`MAX_RECURSION_NODES`・`MAX_LINE_SEGMENTS` は、どれかひとつだけ見ても足りない。
/// 第一が決めるのは **「`depth` という数の最大値」**。とはいえ深さを抑えても、複製（IFS の枝）が多いと **再帰ツリーを踏む回数** や **メッシュに載せる線分の本数** が膨らむ。
/// 第二・第三が、それぞれ **訪問ノード数** と **セグメント総数** の目安で歯止めになる。
///
/// [`max_depth_for_budget`] は `depth` を 1 から順に試し、**第一の上限以下**でも、「訪問が `MAX_RECURSION_NODES` を超える」「`基図形の線分数 × 描画回数` が `MAX_LINE_SEGMENTS` を超える」段階で打ち切る。
/// 要するに **深さ・走査量・線の本数** の三層で予算を切っている。

/// 共有 URL・UI・再帰処理が許容する `depth` の絶対上限。
///
/// この値を超える深さは入力段階で弾き、狭い実行環境でのスタックやタイムアウトを避ける。WASM ではより低い。
///
/// - [`max_depth_for_budget`] は上をすべて満たす最大の `d` を返す（途中で数え上げが溢れそうなら打ち切り）。レプリカが 0 本のときは再帰が無いので、この定数をそのまま返す。
#[cfg(target_arch = "wasm32")]
pub const FRACTAL_DEPTH_HARD_CAP: u32 = 512;
#[cfg(not(target_arch = "wasm32"))]
pub const FRACTAL_DEPTH_HARD_CAP: u32 = 2048;

/// 1 フレームあたり許容する再帰ツリー訪問ノード数の目安上限（メモリ・時間）。
#[cfg(target_arch = "wasm32")]
const MAX_RECURSION_NODES: u64 = 500_000;
#[cfg(not(target_arch = "wasm32"))]
const MAX_RECURSION_NODES: u64 = 2_000_000;

/// 動的メッシュに書き込む線分セグメント数の目安上限（頂点バッファ・GPU）。
#[cfg(target_arch = "wasm32")]
const MAX_LINE_SEGMENTS: u64 = 400_000;
#[cfg(not(target_arch = "wasm32"))]
const MAX_LINE_SEGMENTS: u64 = 1_500_000;

/// 現在の基図形と複製数・描画モードから、予算を破らない範囲で選べる最大の再帰 `depth` を返す。
///
/// 複製が 0 のときは再帰が無いため、[`FRACTAL_DEPTH_HARD_CAP`] をそのまま返す。
///
/// # Arguments
///
/// * `base_line_count` - 基図形の線分数（セグメント数のベース）。
/// * `replica_count` - IFS の枝の本数（各深さでの分岐数）。
/// * `show_all_generations` - `true` のとき途中世代も描画する前提でセグメント数を見積もる。
///
/// # Returns
///
/// `1..=FRACTAL_DEPTH_HARD_CAP` の範囲で、内部上限を超えない最大の `depth`。
pub fn max_depth_for_budget(
    base_line_count: usize,
    replica_count: usize,
    show_all_generations: bool,
) -> u32 {
    if replica_count == 0 {
        return FRACTAL_DEPTH_HARD_CAP;
    }
    let r = replica_count as u64;
    let l = base_line_count as u64;

    let mut best = 1u32;
    for d in 1..=FRACTAL_DEPTH_HARD_CAP {
        let Some(nodes) = recursion_node_count(r, d) else {
            break;
        };
        if nodes > MAX_RECURSION_NODES {
            break;
        }
        let Some(draws) = line_draw_invocations(r, d, show_all_generations) else {
            break;
        };
        let segments = l.saturating_mul(draws);
        if segments > MAX_LINE_SEGMENTS {
            break;
        }
        best = d;
    }
    best
}

/// 分岐数 `r`・深さ `depth` の完全 `r` 分木を、全世代をなぞる場合に訪問するノード数 \( \sum_{i=0}^{depth-1} r^i \)。
///
/// オーバーフローしそうなら `None`。
///
/// # Arguments
///
/// * `r` - 各ノードからの子の数（レプリカ本数）。
/// * `depth` - 再帰の段数（1 なら根のみ）。
///
/// # Returns
///
/// 訪問ノード数。`r == 0` または `depth == 0` のときは `None`。
fn recursion_node_count(r: u64, depth: u32) -> Option<u64> {
    if r == 0 || depth == 0 {
        return None;
    }
    if r == 1 {
        return Some(depth as u64);
    }
    let mut sum = 0u64;
    let mut term = 1u64;
    for _ in 0..depth {
        sum = sum.checked_add(term)?;
        term = term.checked_mul(r)?;
    }
    Some(sum)
}

/// 基図形の各線分をメッシュに何回「載せる」かの見積もり（末端のみ表示 vs 全世代表示）。
///
/// # Arguments
///
/// * `r` - 分岐数（レプリカ本数）。
/// * `depth` - 再帰の深さ。
/// * `show_all_generations` - `true` なら内部ノードでも線分を描く前提の回数。
///
/// # Returns
///
/// 累積セグメント数の計算に使う呼び出し回数。不正な組み合わせでオーバーフローする場合は `None`。
fn line_draw_invocations(r: u64, depth: u32, show_all_generations: bool) -> Option<u64> {
    if depth < 1 {
        return Some(0);
    }
    if show_all_generations {
        recursion_node_count(r, depth)
    } else if depth == 1 {
        Some(1)
    } else {
        let mut t = 1u64;
        for _ in 0..depth - 1 {
            t = t.checked_mul(r)?;
        }
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_replicas_allows_abs_max_depth() {
        assert_eq!(max_depth_for_budget(100, 0, true), FRACTAL_DEPTH_HARD_CAP);
    }

    #[test]
    fn high_branching_clamps_below_hard_cap() {
        let d = max_depth_for_budget(1, 8, false);
        assert!(d < FRACTAL_DEPTH_HARD_CAP);
        assert!(d >= 1);
        let nodes = recursion_node_count(8, d).expect("nodes");
        assert!(nodes <= MAX_RECURSION_NODES);
    }

    #[test]
    fn two_replicas_allow_depth_above_twelve_within_budget() {
        let d = max_depth_for_budget(1, 2, false);
        assert!(d > 12);
        assert!(d <= FRACTAL_DEPTH_HARD_CAP);
    }

    #[test]
    fn recursion_node_count_geometric_series() {
        assert_eq!(recursion_node_count(3, 4), Some(1 + 3 + 9 + 27));
    }

    #[test]
    fn line_draw_invocations_leaf_only() {
        assert_eq!(line_draw_invocations(5, 3, false), Some(25));
    }
}
