//! Unit conversion by dimensional analysis: `convert 5 km to miles`, `convert 100 F to C`,
//! `convert 9.81 m/s^2 to ft/s^2`, `convert 1 kWh to J`.
//!
//! A unit is a vector of integer exponents of the seven SI base dimensions, a scale factor to SI and (for the
//! temperature scales) an offset. Units combine with `*`, `/`, `^` and spaces, and carry SI prefixes (`km`, `mg`,
//! `GHz`). Converting between incompatible dimensions is an error, not a number.

use std::f64::consts::PI;

/// Exponents of L (length), M (mass), T (time), I (current), Θ (temperature), N (amount), J (luminous intensity).
type Dims = [i32; 7];

#[derive(Clone, Copy, Debug)]
struct Unit {
    dims: Dims,
    scale: f64,
    /// Added after scaling: a Celsius value `v` is `v + 273.15` kelvin.
    offset: f64,
}

const NONE: Dims = [0; 7];
const L: Dims = [1, 0, 0, 0, 0, 0, 0];
const M: Dims = [0, 1, 0, 0, 0, 0, 0];
const T: Dims = [0, 0, 1, 0, 0, 0, 0];
const I: Dims = [0, 0, 0, 1, 0, 0, 0];
const TH: Dims = [0, 0, 0, 0, 1, 0, 0];
const N: Dims = [0, 0, 0, 0, 0, 1, 0];
const J: Dims = [0, 0, 0, 0, 0, 0, 1];

fn combine(a: Dims, b: Dims, sign: i32) -> Dims {
    let mut out = a;
    for i in 0..7 {
        out[i] += sign * b[i];
    }
    out
}

fn dim(parts: &[(Dims, i32)]) -> Dims {
    parts.iter().fold(NONE, |acc, &(d, k)| combine(acc, d, k))
}

fn unit(dims: Dims, scale: f64) -> Unit {
    Unit {
        dims,
        scale,
        offset: 0.0,
    }
}

/// Units that take SI prefixes.
fn base_unit(name: &str) -> Option<Unit> {
    let force = dim(&[(M, 1), (L, 1), (T, -2)]);
    let energy = dim(&[(force, 1), (L, 1)]);
    let power = dim(&[(energy, 1), (T, -1)]);
    let pressure = dim(&[(force, 1), (L, -2)]);
    let charge = dim(&[(I, 1), (T, 1)]);
    let volt = dim(&[(power, 1), (I, -1)]);
    Some(match name {
        "m" | "meter" | "metre" => unit(L, 1.0),
        "g" | "gram" => unit(M, 1e-3),
        "s" | "sec" | "second" => unit(T, 1.0),
        "A" | "ampere" | "amp" => unit(I, 1.0),
        "K" | "kelvin" => unit(TH, 1.0),
        "mol" | "mole" => unit(N, 1.0),
        "cd" | "candela" => unit(J, 1.0),
        "N" | "newton" => unit(force, 1.0),
        "Pa" | "pascal" => unit(pressure, 1.0),
        "J" | "joule" => unit(energy, 1.0),
        "W" | "watt" => unit(power, 1.0),
        "C" | "coulomb" => unit(charge, 1.0),
        "V" | "volt" => unit(volt, 1.0),
        "F" | "farad" => unit(dim(&[(charge, 1), (volt, -1)]), 1.0),
        "ohm" | "Ω" => unit(dim(&[(volt, 1), (I, -1)]), 1.0),
        "S" | "siemens" => unit(dim(&[(I, 1), (volt, -1)]), 1.0),
        "Wb" | "weber" => unit(dim(&[(volt, 1), (T, 1)]), 1.0),
        "T" | "tesla" => unit(dim(&[(volt, 1), (T, 1), (L, -2)]), 1.0),
        "H" | "henry" => unit(dim(&[(volt, 1), (T, 1), (I, -1)]), 1.0),
        "Hz" | "hertz" => unit(dim(&[(T, -1)]), 1.0),
        "L" | "l" | "liter" | "litre" => unit(dim(&[(L, 3)]), 1e-3),
        "bar" => unit(pressure, 1e5),
        "eV" => unit(energy, 1.602_176_634e-19),
        "cal" | "calorie" => unit(energy, 4.184),
        "Wh" => unit(energy, 3600.0),
        "rad" | "radian" => unit(NONE, 1.0),
        _ => return None,
    })
}

/// Units that do not take prefixes.
fn plain_unit(name: &str) -> Option<Unit> {
    let force = dim(&[(M, 1), (L, 1), (T, -2)]);
    let pressure = dim(&[(force, 1), (L, -2)]);
    let power = dim(&[(force, 1), (L, 1), (T, -1)]);
    Some(match name {
        "in" | "inch" | "inches" => unit(L, 0.0254),
        "ft" | "foot" | "feet" => unit(L, 0.3048),
        "yd" | "yard" => unit(L, 0.9144),
        "mi" | "mile" => unit(L, 1609.344),
        "nmi" => unit(L, 1852.0),
        "au" | "AU" => unit(L, 149_597_870_700.0),
        "ly" => unit(L, 9_460_730_472_580_800.0),
        "angstrom" | "Å" => unit(L, 1e-10),
        "lb" | "lbs" | "pound" => unit(M, 0.453_592_37),
        "oz" | "ounce" => unit(M, 0.028_349_523_125),
        "tonne" => unit(M, 1000.0),
        "u" | "amu" => unit(M, 1.660_539_066_60e-27),
        "min" | "minute" => unit(T, 60.0),
        "h" | "hr" | "hour" => unit(T, 3600.0),
        "d" | "day" => unit(T, 86400.0),
        "week" => unit(T, 604_800.0),
        "yr" | "year" => unit(T, 31_557_600.0),
        "mph" => unit(dim(&[(L, 1), (T, -1)]), 0.447_04),
        "kn" | "knot" => unit(dim(&[(L, 1), (T, -1)]), 1852.0 / 3600.0),
        "atm" => unit(pressure, 101_325.0),
        "psi" => unit(pressure, 6_894.757_293_168),
        "mmHg" | "torr" => unit(pressure, 133.322_387_415),
        "hp" => unit(power, 745.699_871_582_27),
        "deg" | "degree" | "°" => unit(NONE, PI / 180.0),
        "degC" | "celsius" | "°C" => Unit {
            dims: TH,
            scale: 1.0,
            offset: 273.15,
        },
        "degF" | "fahrenheit" | "°F" => Unit {
            dims: TH,
            scale: 5.0 / 9.0,
            offset: 273.15 - 32.0 * 5.0 / 9.0,
        },
        _ => return None,
    })
}

const PREFIXES: [(&str, f64); 20] = [
    ("Y", 1e24),
    ("Z", 1e21),
    ("E", 1e18),
    ("P", 1e15),
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("h", 1e2),
    ("da", 1e1),
    ("d", 1e-1),
    ("c", 1e-2),
    ("m", 1e-3),
    ("µ", 1e-6),
    ("μ", 1e-6),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
    ("a", 1e-18),
];

fn atom(name: &str) -> Result<Unit, String> {
    // Temperature scales by their usual one-letter names are ambiguous with coulomb and farad: the context decides,
    // and a bare `C` or `F` in a conversion means the temperature scale (`convert 100 F to C`).
    match name {
        "C" => return Ok(plain_unit("degC").unwrap_or(unit(TH, 1.0))),
        "F" => return Ok(plain_unit("degF").unwrap_or(unit(TH, 1.0))),
        _ => {}
    }
    if let Some(u) = plain_unit(name).or_else(|| base_unit(name)) {
        return Ok(u);
    }
    // A plural (`miles`, `seconds`).
    if let Some(singular) = name.strip_suffix('s')
        && let Some(u) = plain_unit(singular).or_else(|| base_unit(singular))
    {
        return Ok(u);
    }
    for (prefix, factor) in PREFIXES {
        if let Some(rest) = name.strip_prefix(prefix)
            && !rest.is_empty()
            && let Some(mut u) =
                base_unit(rest).or_else(|| rest.strip_suffix('s').and_then(base_unit))
        {
            u.scale *= factor;
            return Ok(u);
        }
    }
    Err(format!("unknown unit `{name}`"))
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.pos += 1;
        }
    }

    fn expression(&mut self) -> Result<Unit, String> {
        let mut result = self.term()?;
        loop {
            self.skip_spaces();
            match self.peek() {
                Some('*') | Some('·') => {
                    self.pos += 1;
                    result = multiply(&result, &self.term()?, 1);
                }
                Some('/') => {
                    self.pos += 1;
                    result = multiply(&result, &self.term()?, -1);
                }
                Some(c) if c != ')' => {
                    // Juxtaposition: `kg m / s^2`.
                    result = multiply(&result, &self.term()?, 1);
                }
                _ => return Ok(result),
            }
        }
    }

    fn term(&mut self) -> Result<Unit, String> {
        self.skip_spaces();
        let base = if self.peek() == Some('(') {
            self.pos += 1;
            let inner = self.expression()?;
            self.skip_spaces();
            if self.peek() != Some(')') {
                return Err("a bracket is not closed in the unit".into());
            }
            self.pos += 1;
            inner
        } else {
            let start = self.pos;
            while self
                .peek()
                .is_some_and(|c| c.is_alphabetic() || matches!(c, '°' | 'µ' | 'Ω' | '_'))
            {
                self.pos += 1;
            }
            if start == self.pos {
                return Err("a unit name is expected".into());
            }
            let name: String = self.chars[start..self.pos].iter().collect();
            atom(&name)?
        };
        if self.peek() == Some('^') {
            self.pos += 1;
            let start = self.pos;
            if self.peek() == Some('-') {
                self.pos += 1;
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
            let k: i32 = self.chars[start..self.pos]
                .iter()
                .collect::<String>()
                .parse()
                .map_err(|_| "an integer exponent is expected after ^")?;
            return Ok(power(&base, k));
        }
        Ok(base)
    }
}

fn multiply(a: &Unit, b: &Unit, sign: i32) -> Unit {
    Unit {
        dims: combine(a.dims, b.dims, sign),
        scale: if sign > 0 {
            a.scale * b.scale
        } else {
            a.scale / b.scale
        },
        offset: 0.0,
    }
}

fn power(a: &Unit, k: i32) -> Unit {
    let mut dims = NONE;
    for _ in 0..k.abs() {
        dims = combine(dims, a.dims, 1);
    }
    if k < 0 {
        dims = combine(NONE, dims, -1);
    }
    Unit {
        dims,
        scale: a.scale.powi(k),
        offset: 0.0,
    }
}

fn parse_unit(text: &str) -> Result<Unit, String> {
    let mut parser = Parser {
        chars: text.trim().chars().collect(),
        pos: 0,
    };
    let unit = parser.expression()?;
    if parser.pos != parser.chars.len() {
        return Err(format!("cannot read the unit `{text}`"));
    }
    Ok(unit)
}

fn describe(dims: &Dims) -> String {
    let names = [
        "length",
        "mass",
        "time",
        "current",
        "temperature",
        "amount",
        "luminous intensity",
    ];
    let parts: Vec<String> = dims
        .iter()
        .zip(names)
        .filter(|(k, _)| **k != 0)
        .map(|(k, n)| {
            if *k == 1 {
                n.to_string()
            } else {
                format!("{n}^{k}")
            }
        })
        .collect();
    if parts.is_empty() {
        "dimensionless".into()
    } else {
        parts.join(" · ")
    }
}

/// Convert `value` from one unit to another.
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64, String> {
    let (a, b) = (parse_unit(from)?, parse_unit(to)?);
    if a.dims != b.dims {
        return Err(format!(
            "cannot convert {} ({}) to {} ({}): the dimensions differ",
            from.trim(),
            describe(&a.dims),
            to.trim(),
            describe(&b.dims)
        ));
    }
    Ok(((value * a.scale + a.offset) - b.offset) / b.scale)
}

/// A number with up to ten significant digits and no trailing zeros.
pub fn clean(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return format!("{value}");
    }
    // Very small and very large values in scientific notation (a fixed-point form would lose the digits).
    if value.abs() < 1e-4 || value.abs() >= 1e12 {
        let text = format!("{value:.9e}");
        let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
        let mantissa = if mantissa.contains('.') {
            mantissa.trim_end_matches('0').trim_end_matches('.')
        } else {
            mantissa
        };
        return format!("{mantissa}e{exponent}");
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (9 - magnitude).clamp(0, 15) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}
