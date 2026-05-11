//! 基図形・レプリカ・深さをまとめて切り替えるフラクタル全体プリセット。
//! IFS は各レプリカ R に対し `transform.compose(R)` で深さ方向に合成される。

mod common;
mod hal_cyclone_triangle;
mod hal_mosaic_window;
mod hal_tree;
mod hal_v_star;
mod hal_wing;
mod heighway_dragon;
mod koch_curve;
mod levy_c;
mod pythagoras_tree;
mod sierpinski_hexagon;
mod sierpinski_star;
mod sierpinski_triangle;
mod terdragon;
mod vicsek;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FractalPreset {
    SierpinskiTriangle,
    KochCurve,
    Vicsek,
    HeighwayDragon,
    LevyCCurve,
    PythagorasTree,
    SierpinskiHexagon,
    SierpinskiStar,
    Terdragon,
    HalCycloneTriangle,
    HalWing,
    HalTree,
    HalVStar,
    HalMosaicWindow,
}

impl FractalPreset {
    pub const ALL: &'static [FractalPreset] = &[
        FractalPreset::SierpinskiTriangle,
        FractalPreset::KochCurve,
        FractalPreset::Vicsek,
        FractalPreset::HeighwayDragon,
        FractalPreset::LevyCCurve,
        FractalPreset::PythagorasTree,
        FractalPreset::SierpinskiHexagon,
        FractalPreset::SierpinskiStar,
        FractalPreset::Terdragon,
        FractalPreset::HalCycloneTriangle,
        FractalPreset::HalWing,
        FractalPreset::HalTree,
        FractalPreset::HalVStar,
        FractalPreset::HalMosaicWindow,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FractalPreset::SierpinskiTriangle => "Sierpiński triangle",
            FractalPreset::KochCurve => "Koch curve",
            FractalPreset::Vicsek => "Vicsek (fractal cross)",
            FractalPreset::HeighwayDragon => "Dragon curve",
            FractalPreset::LevyCCurve => "Lévy C curve",
            FractalPreset::PythagorasTree => "Pythagoras tree",
            FractalPreset::SierpinskiHexagon => "Sierpiński hexagon",
            FractalPreset::SierpinskiStar => "Sierpiński pentagram",
            FractalPreset::Terdragon => "Terdragon curve",
            FractalPreset::HalCycloneTriangle => "hal Cyclone Triangle",
            FractalPreset::HalWing => "hal Wing",
            FractalPreset::HalTree => "hal Tree",
            FractalPreset::HalVStar => "hal V Star",
            FractalPreset::HalMosaicWindow => "hal Mosaic Window",
        }
    }

    /// フラクタル定義のみを構築する。
    pub fn build(self) -> crate::app::session::FractalState {
        match self {
            FractalPreset::SierpinskiTriangle => sierpinski_triangle::build(),
            FractalPreset::KochCurve => koch_curve::build(),
            FractalPreset::Vicsek => vicsek::build(),
            FractalPreset::HeighwayDragon => heighway_dragon::build(),
            FractalPreset::LevyCCurve => levy_c::build(),
            FractalPreset::PythagorasTree => pythagoras_tree::build(),
            FractalPreset::SierpinskiHexagon => sierpinski_hexagon::build(),
            FractalPreset::SierpinskiStar => sierpinski_star::build(),
            FractalPreset::Terdragon => terdragon::build(),
            FractalPreset::HalCycloneTriangle => hal_cyclone_triangle::build(),
            FractalPreset::HalWing => hal_wing::build(),
            FractalPreset::HalTree => hal_tree::build(),
            FractalPreset::HalVStar => hal_v_star::build(),
            FractalPreset::HalMosaicWindow => hal_mosaic_window::build(),
        }
    }

    /// スクリーンショット・検証用: 環境変数 `FRACTALIUM_BOOT_PRESET` から解決する。
    pub fn from_boot_token(raw: &str) -> Option<Self> {
        let key: String = raw
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        match key.as_str() {
            "pythagoras" | "pythagorastree" => Some(Self::PythagorasTree),
            "heighway" | "heighwaydragon" | "dragon" => Some(Self::HeighwayDragon),
            _ => Self::ALL.iter().copied().find(|&p| {
                let label_key: String = p
                    .label()
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| c.to_ascii_lowercase())
                    .collect();
                label_key == key
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::session_rules::clamp_fractal_state_depth;

    use super::FractalPreset;

    /// hal 系プリセットの `depth` が予算クランプ後も維持されること。
    #[test]
    fn hal_presets_depth_survives_budget_clamp() {
        for (preset, expected_depth) in [
            (FractalPreset::HalCycloneTriangle, 15u32),
            (FractalPreset::HalWing, 7),
            (FractalPreset::HalTree, 10),
            (FractalPreset::HalVStar, 10),
            (FractalPreset::HalMosaicWindow, 12),
        ] {
            let mut state = preset.build();
            assert_eq!(state.depth, expected_depth);
            clamp_fractal_state_depth(&mut state);
            assert_eq!(
                state.depth, expected_depth,
                "{preset:?}: budget clamp must not lower preset depth on this target"
            );
        }
    }
}
