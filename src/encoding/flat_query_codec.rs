//! `key=value` を `&` で連結した**フラットなクエリ本文**だけをエンコード／デコードする。
//!
//! # サンプル（本文のみ、`?` / `#` なし）
//!
//! ```text
//! v=1&depth=4&g=1&line=-0.5,0.0,0.5,0.0&replica=x:1.0,y:0.0,r:0.0,s:1.0&replica=x:2.0,y:0.0,r:0.0,s:1.0
//! ```
//!
//! [`TopLevel::decode`] すると `(キー, 値)` が左から順に並ぶ。同じキー（例: `replica`）は複数セグメントとしてそのまま残る。
//!
//! # トップレベル書式
//!
//! - セグメントを `&` で連結。各セグメントは `キー=値`。値側のみパーセント復号（[`urlencoding`]）。
//! - `=` のないセグメント・空セグメントは読み飛ばす。
//! - エンコード側は値をそのまま連結する（このモジュールではパーセントエンコードしない）。トークンは ASCII に収める前提。
//!
//! # サブレベル書式（`=` の右・[`SubLevel`] が保持する文字列）
//!
//! - `,` で複数の値を区切り、 HashMap な値を取る場合は `:` で `(サブキー, 値)` に分ける。
//! - 例：
//!   - 単体値： 1
//!   - 数値列： 1.0,-2,3.5
//!   - キー付き値： x:1.0,y:2.0,r:0,s:1


/// エンコード側がサブレベルに書き込む浮動小数トークンで使う有効桁数（桁落ちと文字列長のバランス）。
const WIRE_F32_SIG_FIGS: i32 = 6;
/// エンコード側がサブレベルに書き込む [`f32`] トークンで、有効桁丸めのあと絶対値がこれ未満なら `"0.0"` に寄せる（座標もスケールも同一）。
const WIRE_F32_NEAR_ZERO_EPS: f64 = 1e-7;

/// フラットクエリ本文のトップレベル表現。
///
/// `key=value` を左から右へ並べた順序付き列。値側は [`SubLevel`] がデコード済み文字列として保持する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TopLevel(pub(crate) Vec<(String, SubLevel)>);

impl TopLevel {
    /// `&` 区切りの本文をトップレベル構造へ復号する。
    ///
    /// 各セグメントは `key=value`。値のみパーセント復号する。`=` がないセグメント・空セグメントは読み飛ばす。
    ///
    /// # 引数
    /// - `query_body` — `?` / `#` を含まないクエリ風本文。
    ///
    /// # 戻り値
    /// 復号に成功した [`TopLevel`]。
    pub(crate) fn decode(query_body: &str) -> Result<Self, String> {
        let mut out = Vec::new();
        for raw_seg in query_body.split('&') {
            if raw_seg.is_empty() {
                continue;
            }
            let seg = raw_seg.trim();
            let Some((key, raw_val)) = seg.split_once('=') else {
                continue;
            };
            let val = urlencoding::decode(raw_val)
                .map_err(|e| e.to_string())?
                .into_owned();
            out.push((key.to_string(), SubLevel(val)));
        }
        Ok(Self(out))
    }

    /// [`Self::pairs`] の並びを `key=value&…` の本文へエンコードする。この関数では値をパーセントエンコードしない。
    ///
    /// # 戻り値
    /// `&` で連結した文字列（先頭に `?` / `#` は付けない）。
    pub(crate) fn encode(&self) -> String {
        self.0
            .iter()
            .map(|(k, v)| format!("{k}={}", v.as_str()))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// キーとサブレベル値の並びへの参照を返す。
    ///
    /// # 戻り値
    /// ワイヤの左から右への順序のスライス。
    pub(crate) fn pairs(&self) -> &[(String, SubLevel)] {
        &self.0
    }

    /// **連続して**現れる同一キーの値だけを [`Vec`] にまとめる（キーが交互に現れる場合はエントリが分かれる）。
    ///
    /// # 戻り値
    /// `(キー, そのキーの値の列)`。出現順は維持される。
    #[cfg(test)]
    fn group_values_by_key(&self) -> Vec<(String, Vec<SubLevel>)> {
        let mut out: Vec<(String, Vec<SubLevel>)> = Vec::new();
        for (k, v) in &self.0 {
            match out.last_mut() {
                Some((gk, gv)) if gk == k => gv.push(v.clone()),
                _ => out.push((k.clone(), vec![v.clone()])),
            }
        }
        out
    }

    /// 同一キーに複数値を載せるときの `(キー, 値)` 列を組み立てる（[`Self::group_values_by_key`] の逆に相当）。
    ///
    /// # 引数
    /// - `key` — 繰り返すトップレベルキー。
    /// - `values` — 各要素が別々の `key=value` の値側になる。
    ///
    /// # 戻り値
    /// [`TopLevel`] を構築するのに使える `(キー, [`SubLevel`])` のベクトル。
    #[cfg(test)]
    fn expand_repeated_key(
        key: impl AsRef<str>,
        values: &[impl AsRef<str>],
    ) -> Vec<(String, SubLevel)> {
        let key = key.as_ref();
        values
            .iter()
            .map(|v| (key.to_string(), SubLevel(v.as_ref().to_string())))
            .collect()
    }
}

/// トップレベルで `=` の右に来る値。パーセント復号済みのワイヤ文字列を保持する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubLevel(pub(crate) String);

impl SubLevel {
    /// 値側文字列から構築する（通常は [`TopLevel::decode`] が復号したもの）。
    ///
    /// # 引数
    /// - `s` — サブレベル値の字句全体。
    ///
    /// # 戻り値
    /// ラップした [`SubLevel`]。
    pub(crate) fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// 保持しているワイヤ文字列を参照する。
    ///
    /// # 戻り値
    /// デコード済みの UTF-8 スライス。
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// サブレベル値をトークン列として解釈し、各要素を [`f32`] に読む。
    ///
    /// # 戻り値
    /// パースに成功した [`f32`] のベクトル。無効トークンがあると `Err`。
    pub(crate) fn decode_to_f32_vec(&self) -> Result<Vec<f32>, String> {
        let tokens = self.decode_to_str_vec();
        let mut out = Vec::with_capacity(tokens.len());
        for t in tokens {
            let x = t
                .parse::<f32>()
                .map_err(|_| format!("invalid f32 token: {t}"))?;
            out.push(x);
        }
        Ok(out)
    }

    /// 順序付き `(サブキー, 値)` の列として復号する。
    ///
    /// # 戻り値
    /// [`SubLevelKv`]。構文が規約に合わないときは `Err`。
    pub(crate) fn decode_to_kv_pairs(&self) -> Result<SubLevelKv, String> {
        let mut out = Vec::new();
        for raw in self.as_str().split(',') {
            let seg = raw.trim();
            if seg.is_empty() {
                continue;
            }
            let Some((k, v)) = seg.split_once(':') else {
                return Err(format!("colon-kv segment missing ':' ({seg})"));
            };
            let k = k.trim();
            if k.is_empty() {
                return Err(format!("colon-kv empty sub-key ({seg})"));
            }
            out.push((k.to_string(), v.trim().to_string()));
        }
        Ok(SubLevelKv(out))
    }

    /// 値全体をトリムし、単一の非負整数として復号する。
    ///
    /// # 戻り値
    /// パースした [`u32`]。形式が合わないときは `Err`。
    pub(crate) fn decode_to_u32(&self) -> Result<u32, String> {
        self.as_str()
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("bad u32: {}", self.as_str()))
    }

    /// 値全体をトリムし、`0` / `1` / `true` / `false` のいずれかとして復号する（この4種のみ、大小はこのまま）。
    ///
    /// # 戻り値
    /// 復号した [`bool`]。
    pub(crate) fn decode_to_bool_bin(&self) -> Result<bool, String> {
        match self.as_str().trim() {
            "1" | "true" => Ok(true),
            "0" | "false" => Ok(false),
            _ => Err(format!("bad bool: {}", self.as_str())),
        }
    }

    /// 順番どおり [`f32`] を `,` 区切りのサブレベル値へ書き出す。
    ///
    /// 各要素は [`WIRE_F32_SIG_FIGS`]・[`WIRE_F32_NEAR_ZERO_EPS`] に従いトークン化する（[`SubLevel::encode_from_kv_f32`] と同一規約）。
    ///
    /// # 引数
    /// - `values` — 左から右への数値。[`SubLevel::decode_to_f32_vec`] と往復させる読み側の構文に合わせたいときに使う。
    ///
    /// # 戻り値
    /// 結合した [`SubLevel`]（`values` が空なら空文字列のサブレベル）。
    pub(crate) fn encode_from_f32_vec(values: &[f32]) -> Self {
        Self(
            values
                .iter()
                .copied()
                .map(|f| Self::encode_wire_f32_token(f as f64))
                .collect::<Vec<_>>()
                .join(","),
        )
    }

    /// [`SubLevel::decode_to_kv_pairs`] が読める形で、順序どおり `(サブキー, [`f32`])` を書き出す。
    ///
    /// 値側は [`Self::encode_from_f32_vec`] の 1 トークンと同じ規約でエンコードする。
    ///
    /// # 引数
    /// - `entries` — `,` で結ばれるセグメントの並び。
    ///
    /// # 戻り値
    /// 結合した [`SubLevel`]（`entries` が空なら空文字列のサブレベル）。
    pub(crate) fn encode_from_kv_f32(entries: &[(&str, f32)]) -> Self {
        Self(
            entries
                .iter()
                .map(|(k, f)| format!("{}:{}", k, Self::encode_wire_f32_token(*f as f64)))
                .collect::<Vec<_>>()
                .join(","),
        )
    }

    /// 1 個の浮動小数を、この型が保持するサブレベル値の途中に置く ASCII トークン 1 個へ変換する。
    ///
    /// [`WIRE_F32_SIG_FIGS`] で丸めたあとの絶対値が [`WIRE_F32_NEAR_ZERO_EPS`] 未満なら `"0.0"` に寄せる（浮動小数ノイズや極小スケールをワイヤ上で畳む。ドメイン上スケールはここより大きい前提）。
    ///
    /// # 引数
    /// - `v0` — エンコードする値（内部では [`f64`] に上げてから処理する）。
    ///
    /// # 戻り値
    /// 非有限は `"nan"`。有限は小数点を含むトークン。
    fn encode_wire_f32_token(v0: f64) -> String {
        if !v0.is_finite() {
            return "nan".to_string();
        }
        let v = Self::round_f64_to_sig_figs(v0, WIRE_F32_SIG_FIGS);
        if v.abs() < WIRE_F32_NEAR_ZERO_EPS {
            return "0.0".to_string();
        }
        let abs_v = v.abs();
        let log10 = abs_v.log10();
        if !log10.is_finite() {
            return "0.0".to_string();
        }
        let m_i = log10.floor() as i32;
        let frac_digits = ((WIRE_F32_SIG_FIGS - 1) - m_i).max(0).min(20) as usize;
        let rendered = format!("{:.*}", frac_digits, v);
        Self::ensure_wire_float_has_decimal_point(
            &Self::trim_trailing_fraction_zeros(&rendered),
        )
    }

    /// 有効桁 `sig` に丸めた [`f64`] を返す。`0` または非有限は入力をそのまま返す。
    ///
    /// # 引数
    /// - `x` — 丸める値。
    /// - `sig` — 有効桁数（1 以上を想定）。
    ///
    /// # 戻り値
    /// 丸め後の `f64`。
    fn round_f64_to_sig_figs(x: f64, sig: i32) -> f64 {
        if x == 0.0 || !x.is_finite() {
            return x;
        }
        let abs_x = x.abs();
        let log10 = abs_x.log10();
        if !log10.is_finite() {
            return x;
        }
        let m = log10.floor();
        let scale = 10_f64.powf(sig as f64 - 1.0 - m);
        (x * scale).round() / scale
    }

    /// 末尾の冗長な `0` と、孤立した `.` を削ってトークンを短くする（`format!` の余り桁の整理）。
    ///
    /// # 引数
    /// - `s` — 小数点を含みうる十進文字列。
    ///
    /// # 戻り値
    /// トリム後の文字列。`-` だけになる場合は `"0"` に置き換える。
    fn trim_trailing_fraction_zeros(s: &str) -> String {
        if !s.contains('.') {
            return s.to_string();
        }
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s.is_empty() || s == "-" {
            "0".to_string()
        } else {
            s.to_string()
        }
    }

    /// 整数だけの見た目（例: `3`）に `.0` を付け、[`SubLevel::decode_to_f32_vec`] で読むときに馴染むトークン形にそろえる。
    ///
    /// # 引数
    /// - `s` — 丸め・末尾トリム済みのトークン候補。
    ///
    /// # 戻り値
    /// 小数点を含む文字列。入力が `"nan"` ならそのまま。
    fn ensure_wire_float_has_decimal_point(s: &str) -> String {
        if s == "nan" || s.contains('.') {
            return s.to_string();
        }
        if s.is_empty() || s == "-" {
            "0.0".to_string()
        } else {
            format!("{s}.0")
        }
    }

    /// [`SubLevel::decode_to_f32_vec`] と共有するトークン分割（`,` で区切り、前後空白除去・空要素は捨てる）。
    ///
    /// # 戻り値
    /// 左から右へのトークン。
    fn decode_to_str_vec(&self) -> Vec<String> {
        self.as_str()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }
}

/// [`SubLevel::decode_to_kv_pairs`] の結果型。順序付き `(サブキー, 値)` を保持する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubLevelKv(pub(crate) Vec<(String, String)>);

impl SubLevelKv {
    /// 先頭から走査し、最初に一致したサブキーの値を [`f32`] にする。
    ///
    /// # 引数
    /// - `sub_key` — 検索するサブキー。
    ///
    /// # 戻り値
    /// パースに成功した [`f32`]。キーが無い・数値でない場合は `Err`。
    pub(crate) fn get_f32(&self, sub_key: &str) -> Result<f32, String> {
        for (k, v) in &self.0 {
            if k == sub_key {
                return v
                    .parse::<f32>()
                    .map_err(|_| format!("bad f32 for {sub_key}: {v}"));
            }
        }
        Err(format!("missing sub-key {sub_key}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip_preserves_pairs() {
        let top = TopLevel(vec![
            ("v".into(), SubLevel::new("1")),
            ("depth".into(), SubLevel::new("3")),
            ("line".into(), SubLevel::new("1.0,2.0,3.0,4.0")),
        ]);
        let s = top.encode();
        let back = TopLevel::decode(&s).unwrap();
        assert_eq!(back, top);
    }

    #[test]
    fn decode_percent_encoded_value() {
        let top = TopLevel::decode("k=a%26b").unwrap();
        assert_eq!(
            top.pairs(),
            &[("k".into(), SubLevel::new("a&b"))]
        );
    }

    #[test]
    fn repeated_key_roundtrip() {
        let top = TopLevel(vec![
            ("v".into(), SubLevel::new("1")),
            ("replica".into(), SubLevel::new("x:1,y:1,r:0,s:1")),
            ("replica".into(), SubLevel::new("x:2,y:2,r:0,s:1")),
        ]);
        let s = top.encode();
        let back = TopLevel::decode(&s).unwrap();
        assert_eq!(back, top);
        let grouped = back.group_values_by_key();
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, "v");
        assert_eq!(grouped[1].1.len(), 2);
    }

    #[test]
    fn encode_decode_f32_vec_roundtrip() {
        let sl = SubLevel::encode_from_f32_vec(&[1.0, 2.0, 3.5]);
        assert_eq!(sl.decode_to_f32_vec().unwrap(), vec![1.0_f32, 2.0, 3.5]);
    }

    #[test]
    fn comma_f32_parse() {
        assert_eq!(
            SubLevel::new("1.0, -2 , 3.5")
                .decode_to_f32_vec()
                .unwrap(),
            vec![1.0_f32, -2.0, 3.5]
        );
    }

    #[test]
    fn sub_level_kv_roundtrip() {
        let entries = SubLevelKv(vec![
            ("x".into(), "1.0".into()),
            ("y".into(), "2.0".into()),
        ]);
        let sl = SubLevel::encode_from_kv_f32(&[("x", 1.0), ("y", 2.0)]);
        assert_eq!(sl.decode_to_kv_pairs().unwrap(), entries);
    }

    #[test]
    fn sub_level_kv_get_f32() {
        let w = SubLevel::new("x:1,y:2,r:0,s:0.5,z:9")
            .decode_to_kv_pairs()
            .unwrap();
        assert_eq!(w.get_f32("x").unwrap(), 1.0);
        assert_eq!(w.get_f32("s").unwrap(), 0.5);
    }

    #[test]
    fn expand_and_encode_repeated() {
        let ex = TopLevel::expand_repeated_key("replica", &["a", "b"]);
        assert_eq!(TopLevel(ex).encode(), "replica=a&replica=b");
    }

    #[test]
    fn wire_u32_and_bool() {
        assert_eq!(SubLevel::new(" 42 ").decode_to_u32().unwrap(), 42);
        assert!(SubLevel::new("1").decode_to_bool_bin().unwrap());
        assert!(!SubLevel::new("false").decode_to_bool_bin().unwrap());
    }

    #[test]
    fn wire_f32_six_sig_figs_trims_trailing_zeros() {
        assert_eq!(SubLevel::encode_from_f32_vec(&[3.10000002f32]).as_str(), "3.1");
    }

    #[test]
    fn wire_f32_decimal_point_in_token() {
        assert_eq!(SubLevel::encode_from_f32_vec(&[3.0f32]).as_str(), "3.0");
        assert_eq!(
            SubLevel::encode_from_kv_f32(&[("k", 1.0f32)]).as_str(),
            "k:1.0"
        );
    }

    #[test]
    fn wire_f32_tiny_noise_clamps_to_zero() {
        assert_eq!(SubLevel::encode_from_f32_vec(&[-0.000000044f32]).as_str(), "0.0");
    }

    #[test]
    fn wire_f32_kv_keeps_small_positive_scale() {
        assert_eq!(
            SubLevel::encode_from_kv_f32(&[("s", 0.05f32)]).as_str(),
            "s:0.05"
        );
    }
}
