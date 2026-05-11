//! 基図形・レプリカ・深さをまとめて切り替えるフラクタル全体プリセット。
//! IFS は各レプリカ R に対し `transform.compose(R)` で深さ方向に合成される。

mod binary_fractal_tree;
mod common;
mod heighway_dragon;
mod koch_curve;
mod levy_c;
mod pythagoras_tree;
mod sierpinski_carpet;
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
    SierpinskiCarpet,
    PythagorasTree,
    SierpinskiHexagon,
    SierpinskiStar,
    BinaryFractalTree,
    Terdragon,
}

impl FractalPreset {
    pub const ALL: &'static [FractalPreset] = &[
        FractalPreset::SierpinskiTriangle,
        FractalPreset::KochCurve,
        FractalPreset::Vicsek,
        FractalPreset::HeighwayDragon,
        FractalPreset::LevyCCurve,
        FractalPreset::SierpinskiCarpet,
        FractalPreset::PythagorasTree,
        FractalPreset::SierpinskiHexagon,
        FractalPreset::SierpinskiStar,
        FractalPreset::BinaryFractalTree,
        FractalPreset::Terdragon,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FractalPreset::SierpinskiTriangle => "Sierpiński triangle",
            FractalPreset::KochCurve => "Koch curve",
            FractalPreset::Vicsek => "Vicsek (fractal cross)",
            FractalPreset::HeighwayDragon => "Dragon curve",
            FractalPreset::LevyCCurve => "Lévy C curve",
            FractalPreset::SierpinskiCarpet => "Sierpiński carpet",
            FractalPreset::PythagorasTree => "Pythagoras tree",
            FractalPreset::SierpinskiHexagon => "Sierpiński hexagon",
            FractalPreset::SierpinskiStar => "Sierpiński pentagram",
            FractalPreset::BinaryFractalTree => "Binary fractal tree",
            FractalPreset::Terdragon => "Terdragon curve",
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
            FractalPreset::SierpinskiCarpet => sierpinski_carpet::build(),
            FractalPreset::PythagorasTree => pythagoras_tree::build(),
            FractalPreset::SierpinskiHexagon => sierpinski_hexagon::build(),
            FractalPreset::SierpinskiStar => sierpinski_star::build(),
            FractalPreset::BinaryFractalTree => binary_fractal_tree::build(),
            FractalPreset::Terdragon => terdragon::build(),
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
