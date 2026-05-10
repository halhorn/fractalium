//! [`BaseShape`]・[`Line`]・[`Replica`] とレプリカスケール定数により、
//! フラクタルの形状と IFS の各枝変換を表すデータモデルを定める。
//!
//! メニューやドラッグで構成を組み替えても、共有リンクや復元処理を経ても、
//! 線分段の並びと枝への相似変換の解釈を同じ規則に揃える。
//! 利用者側では、画面上の編集結果と復元結果のぶれが出にくい。
//!
//! 開発側では `apply`、`inverse_apply`、`compose` が変換の適用順と合成規約を固定し、
//! メッシュや当たり、再帰走査などが共通の前提を読める。
//! また `REPLICA_SCALE_MIN` / `MAX` は UI・入力検証・共有形式でのスケール境界をひとつの場所に置くために定める。

use glam::Vec2;

/// UI・入力検証・共有などが共通で参照する、レプリカの一様スケール下限。
///
/// 極端な縮小を防ぎ、数値計算や意図しない見え方を抑えるために設ける。
pub const REPLICA_SCALE_MIN: f32 = 0.05;

/// UI・入力検証・共有などが共通で参照する、レプリカの一様スケール上限。
///
/// 極端な拡大でキャンバスが破綻したり検証だけが緩んだりしないよう上限を統一する。
pub const REPLICA_SCALE_MAX: f32 = 2.0;

/// 二次元に向きを持って置かれる線分。基図形はこの要素の並びとして表される。
///
/// `a` から `b` が端点であり、ドラッグ・描画ともにその向きを保つ前提で読む。
#[derive(Clone, Copy)]
pub struct Line {
    /// 始点（基図形座標。[-1, 1] に収める運用が一般的）。
    pub a: Vec2,
    /// 終点。
    pub b: Vec2,
}

/// IFS の再帰が参照する基図形。線分段の並びのみをユーザーの編集状態から切り離して保持する。
#[derive(Default, Clone)]
pub struct BaseShape {
    /// 線分リスト。長さが 0 のときは図形として未設定を意味する運用側の解釈に任せる。
    pub lines: Vec<Line>,
}

/// IFS の 1 本の枝として基図形空間に作用する相似変換。適用順は一様スケール → 回転 → [`Replica::position`] を足す。
///
/// `compose` は親の適用後に子を載せる積であり、ツリーを深める方向との対応からこの順になる。
#[derive(Clone, Copy)]
pub struct Replica {
    /// 配置位置。基図形の原点の配置先。[`Replica::apply`] でスケール、回転のあとに適用される。
    pub position: Vec2,
    /// 回転（ラジアン、反時計回りが正）。
    pub rotation: f32,
    /// 一様スケール（正であることを前提とする）。
    pub scale: f32,
}

impl Replica {
    /// UI が新規に枝を増やしたときの初期値。見えやすく、かつキャンバスから大きく外れにくい目安として使う。
    ///
    /// # Returns
    ///
    /// `position` がゼロ、回転がゼロ、`scale` が `0.5` の [`Replica`]。
    pub fn default_new() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: 0.5,
        }
    }

    /// ツリーのルートで使う恒等変換。親側から見た「このノードでの追加変換がない」を表現するために使う。
    ///
    /// # Returns
    ///
    /// `position` および回転がゼロで `scale` が `1` の [`Replica`]。
    pub fn identity() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: 1.0,
        }
    }

    /// 基図形ローカルの点 `p` に、この枝の適用順（スケール → 回転 → [`Replica::position`] の加算）をそろえた結果の点。
    ///
    /// 描画や当たりで枝を親の適用済み座標へ順に載せ続けるときの中心処理となる。
    ///
    /// # Arguments
    ///
    /// * `p` - 変換前の点（基図形ローカル座標）。
    ///
    /// # Returns
    ///
    /// 変換後の点。
    pub fn apply(self, p: Vec2) -> Vec2 {
        let scaled = p * self.scale;
        let (sin, cos) = self.rotation.sin_cos();
        Vec2::new(
            scaled.x * cos - scaled.y * sin,
            scaled.x * sin + scaled.y * cos,
        ) + self.position
    }

    /// [`apply`](Self::apply) と逆方向の変換として、画面上で得られた点から基図形ローカルへ戻す。
    ///
    /// ドラッグ中のヒットテストや、子レイヤでの座標照合などに必要になる。
    ///
    /// # Arguments
    ///
    /// * `p` - 変換後の点。[`apply`](Self::apply) の出力側と見なせるように置く。
    ///
    /// # Returns
    ///
    /// 対応する基図形ローカルの点。`scale == 0` のときは除算により値が定まらず、呼び出し側では避ける。
    pub fn inverse_apply(self, p: Vec2) -> Vec2 {
        let q = p - self.position;
        let (sin, cos) = self.rotation.sin_cos();
        let unrotated = Vec2::new(q.x * cos + q.y * sin, -q.x * sin + q.y * cos);
        unrotated / self.scale
    }

    /// まず `other` を適用してから続けて `self` を適用したときと同じ結果を、ひとつの [`Replica`] として書き直す。
    ///
    /// 深さをたどって積んだ枝を、ひとつの合成変換としてまとめられるので再帰や描画側の共通実装へ渡しやすい。
    ///
    /// たとえば `self.compose(other).apply(p)` は `self.apply(other.apply(p))` と一致する。
    ///
    /// # Arguments
    ///
    /// * `other` - 内側（先に基図形点に適用する変換）。
    ///
    /// # Returns
    ///
    /// 合成されたレプリカ。
    pub fn compose(self, other: Replica) -> Replica {
        let (sin_s, cos_s) = self.rotation.sin_cos();
        let rot_t = Vec2::new(
            other.position.x * cos_s - other.position.y * sin_s,
            other.position.x * sin_s + other.position.y * cos_s,
        );
        Replica {
            position: self.position + self.scale * rot_t,
            rotation: self.rotation + other.rotation,
            scale: self.scale * other.scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_apply_is_noop() {
        let id = Replica::identity();
        let p = Vec2::new(0.3, -0.7);
        assert_eq!(id.apply(p), p);
    }

    #[test]
    fn inverse_apply_roundtrips_apply() {
        let r = Replica {
            position: Vec2::new(0.1, -0.2),
            rotation: 0.37,
            scale: 0.83,
        };
        let p = Vec2::new(0.4, 0.55);
        let w = r.apply(p);
        let back = r.inverse_apply(w);
        assert!(
            (back - p).length() < 1e-5,
            "got {:?} expected {:?}",
            back,
            p
        );
    }

    #[test]
    fn compose_matches_sequential_apply() {
        let a = Replica {
            position: Vec2::new(0.2, 0.0),
            rotation: 0.25,
            scale: 0.9,
        };
        let b = Replica {
            position: Vec2::new(-0.1, 0.15),
            rotation: -0.1,
            scale: 1.1,
        };
        let c = a.compose(b);
        let p = Vec2::new(0.33, -0.44);
        assert!((c.apply(p) - a.apply(b.apply(p))).length() < 1e-5);
    }

    #[test]
    fn default_new_scale_is_half() {
        let r = Replica::default_new();
        assert_eq!(r.scale, 0.5);
        assert_eq!(r.rotation, 0.0);
    }
}
