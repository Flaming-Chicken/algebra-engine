//! Friendlier input: LaTeX and plain English are rewritten into the engine's own syntax before parsing.
//!
//! LaTeX: `\frac{a}{b}`, `\sqrt[n]{x}`, `\int_a^b f dx`, `\sum_{k=a}^{b} f`, `\prod`, `\lim_{x \to a} f`, `\frac{d}{dx} f`,
//! `\binom{n}{k}`, `\log_b x`, `\sin^2 x`, Greek letters, `\infty`, matrices (`pmatrix`, `bmatrix`) and `\det`.
//!
//! English: `what is 2+2`, `calculate 3*4`, `2 plus 3`, `10 divided by 4`, `3 squared`, `2 to the power 10`,
//! `square root of 16`, `factorial of 5`, `20 percent of 150`.

/// A balanced group starting at `chars[i]` (`{`, `(` or `[`): its content and the index after it.
fn group(chars: &[char], i: usize) -> Option<(String, usize)> {
    let open = *chars.get(i)?;
    let close = match open {
        '{' => '}',
        '(' => ')',
        '[' => ']',
        _ => return None,
    };
    let mut depth = 0;
    for (j, &c) in chars.iter().enumerate().skip(i) {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some((chars[i + 1..j].iter().collect(), j + 1));
            }
        }
    }
    None
}

fn skip_spaces(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// A script (`_x`, `_{...}`, `^2`, `^{...}`) after a command: its converted content and the next index.
fn script(chars: &[char], i: usize, marker: char) -> Option<(String, usize)> {
    let at = skip_spaces(chars, i);
    if chars.get(at) != Some(&marker) {
        return None;
    }
    let after = at + 1;
    if chars.get(after) == Some(&'{') {
        let (inner, next) = group(chars, after)?;
        return Some((convert(&inner), next));
    }
    if chars.get(after) == Some(&'\\') {
        let mut j = after + 1;
        while j < chars.len() && chars[j].is_alphabetic() {
            j += 1;
        }
        return Some((convert(&chars[after..j].iter().collect::<String>()), j));
    }
    let mut j = after;
    if chars.get(j) == Some(&'-') {
        j += 1;
    }
    while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '.') {
        j += 1;
        // A single token: `^2x` is `^2` then `x`.
        if !chars[j - 1].is_ascii_digit() && !chars[j - 1].is_ascii_alphabetic() {
            break;
        }
        if chars[after].is_alphabetic() {
            break;
        }
    }
    if j == after {
        return None;
    }
    Some((chars[after..j].iter().collect(), j))
}

fn function_names() -> &'static [&'static str] {
    &[
        "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh",
        "tanh", "exp", "ln", "log", "det", "gcd", "lcm", "max", "min", "arg", "sign",
    ]
}

fn greek(name: &str) -> Option<&'static str> {
    Some(match name {
        "pi" => "pi",
        "alpha" => "alpha",
        "beta" => "beta",
        "gamma" => "gamma",
        "delta" => "delta",
        "epsilon" | "varepsilon" => "epsilon",
        "theta" => "theta",
        "lambda" => "lambda",
        "mu" => "mu",
        "sigma" => "sigma",
        "phi" | "varphi" => "phi",
        "omega" => "omega",
        "tau" => "tau",
        "rho" => "rho",
        "infty" => "oo",
        "cdot" | "times" | "ast" => "*",
        "div" => "/",
        "le" | "leq" => "<=",
        "ge" | "geq" => ">=",
        "ne" | "neq" => "!=",
        "to" | "rightarrow" => "->",
        "ldots" | "cdots" | "dots" => "...",
        "left" | "right" | "displaystyle" | "quad" | "qquad" | "," | ";" | "!" => "",
        _ => return None,
    })
}

/// The argument of a function written without brackets (`\sin x`, `\ln 2x`): a number or identifier run.
fn bare_argument(chars: &[char], i: usize) -> (String, usize) {
    let at = skip_spaces(chars, i);
    match chars.get(at) {
        Some('(') => {
            if let Some((inner, next)) = group(chars, at) {
                return (convert(&inner), next);
            }
            (String::new(), at)
        }
        Some('{') => {
            if let Some((inner, next)) = group(chars, at) {
                return (convert(&inner), next);
            }
            (String::new(), at)
        }
        Some('\\') => {
            // A command such as `\pi` or `\frac{..}{..}`.
            let mut j = at + 1;
            while j < chars.len() && chars[j].is_alphabetic() {
                j += 1;
            }
            let name: String = chars[at + 1..j].iter().collect();
            if name == "begin"
                && let Some((environment, after_begin)) = group(chars, j)
            {
                let end_marker: Vec<char> = format!(r"\end{{{environment}}}").chars().collect();
                if let Some(offset) = chars[after_begin..]
                    .windows(end_marker.len())
                    .position(|w| w == end_marker.as_slice())
                {
                    let end = after_begin + offset + end_marker.len();
                    let text: String = chars[at..end].iter().collect();
                    return (convert(&text), end);
                }
            }
            if name == "frac" || name == "sqrt" {
                let mut end = j;
                for _ in 0..2 {
                    if let Some((_, next)) = group(chars, skip_spaces(chars, end)) {
                        end = next;
                    }
                }
                let text: String = chars[at..end].iter().collect();
                return (convert(&text), end);
            }
            (convert(&chars[at..j].iter().collect::<String>()), j)
        }
        _ => {
            let mut j = at;
            if chars.get(j) == Some(&'-') {
                j += 1;
            }
            // A coefficient and one identifier (`2x`), or a number, or an identifier.
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j < chars.len()
                && chars[j].is_ascii_alphabetic()
                && (j == at || chars[at..j].iter().all(|c| c.is_ascii_digit() || *c == '-'))
            {
                j += 1;
            }
            (chars[at..j].iter().collect(), j)
        }
    }
}

/// Convert LaTeX to the engine's syntax.
fn convert(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            // A plain brace group (an exponent, a subscript, a grouping).
            if let Some((inner, next)) = group(&chars, i) {
                out.push('(');
                out.push_str(&convert(&inner));
                out.push(')');
                i = next;
                continue;
            }
        }
        if c != '\\' {
            out.push(c);
            i += 1;
            continue;
        }
        // A command.
        let mut j = i + 1;
        if j < chars.len() && !chars[j].is_alphabetic() {
            // `\,`, `\;`, `\!`, `\\` and escaped symbols.
            let token: String = chars[j].to_string();
            if let Some(g) = greek(&token) {
                out.push_str(g);
            } else if chars[j] == '\\' {
                out.push_str("],[");
            } else {
                out.push(chars[j]);
            }
            i = j + 1;
            continue;
        }
        while j < chars.len() && chars[j].is_alphabetic() {
            j += 1;
        }
        let name: String = chars[i + 1..j].iter().collect();
        let rest = j;
        match name.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let at = skip_spaces(&chars, rest);
                if let Some((numerator, after_n)) = group(&chars, at)
                    && let Some((denominator, after_d)) =
                        group(&chars, skip_spaces(&chars, after_n))
                {
                    // d/dx and d^2/dx^2: a derivative of what follows.
                    let n = numerator.trim();
                    let d = denominator.trim();
                    let order = if n == "d" {
                        Some(1)
                    } else {
                        n.strip_prefix("d^").and_then(|e| {
                            e.trim_matches(|ch| ch == '{' || ch == '}')
                                .parse::<u32>()
                                .ok()
                        })
                    };
                    if let Some(order) = order
                        && let Some(variable) = d.strip_prefix('d')
                    {
                        let variable: String = variable
                            .chars()
                            .take_while(|ch| ch.is_alphanumeric())
                            .collect();
                        if !variable.is_empty() {
                            let body: String = chars[after_d..].iter().collect();
                            out.push_str(&format!(
                                "diff({}, {variable}, {order})",
                                latex(body.trim())
                            ));
                            return out;
                        }
                    }
                    out.push_str(&format!(
                        "({}/{})",
                        wrap(&convert(&numerator)),
                        wrap(&convert(&denominator))
                    ));
                    i = after_d;
                    continue;
                }
            }
            "sqrt" => {
                let at = skip_spaces(&chars, rest);
                let (index, after_index) = match group(&chars, at) {
                    Some((inner, next)) if chars[at] == '[' => (Some(convert(&inner)), next),
                    _ => (None, at),
                };
                if let Some((radicand, next)) = group(&chars, skip_spaces(&chars, after_index)) {
                    match index {
                        Some(n) => out.push_str(&format!("root({}, {n})", convert(&radicand))),
                        None => out.push_str(&format!("sqrt({})", convert(&radicand))),
                    }
                    i = next;
                    continue;
                }
            }
            "binom" | "dbinom" | "tbinom" => {
                let at = skip_spaces(&chars, rest);
                if let Some((n, after_n)) = group(&chars, at)
                    && let Some((k, after_k)) = group(&chars, skip_spaces(&chars, after_n))
                {
                    out.push_str(&format!("binomial({}, {})", convert(&n), convert(&k)));
                    i = after_k;
                    continue;
                }
            }
            "int" | "sum" | "prod" | "lim" => {
                let (lower, after_lower) =
                    script(&chars, rest, '_').unwrap_or((String::new(), rest));
                let (upper, after_upper) =
                    script(&chars, after_lower, '^').unwrap_or((String::new(), after_lower));
                let body_chars = &chars[after_upper..];
                let body: String = body_chars.iter().collect();
                let strip = |s: &str| {
                    s.trim()
                        .trim_start_matches('(')
                        .trim_end_matches(')')
                        .to_string()
                };
                match name.as_str() {
                    "int" => {
                        // The integrand ends at `dx`.
                        let text = body.trim();
                        let (integrand, variable) = split_differential(text);
                        let f = latex(&integrand);
                        if lower.is_empty() && upper.is_empty() {
                            out.push_str(&format!("integrate({f}, {variable})"));
                        } else {
                            out.push_str(&format!(
                                "integrate({f}, {variable}, {}, {})",
                                strip(&lower),
                                strip(&upper)
                            ));
                        }
                    }
                    "sum" | "prod" => {
                        let command = if name == "sum" { "sum" } else { "product" };
                        let (summand, tail) = split_scope(body.trim());
                        let tail = if tail.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", latex(&tail))
                        };
                        if let Some((variable, start)) = lower.split_once('=') {
                            let end = if upper.is_empty() {
                                "oo".to_string()
                            } else {
                                strip(&upper)
                            };
                            out.push_str(&format!(
                                "{command}({}, {}, {}, {end}){tail}",
                                latex(&summand),
                                strip(variable),
                                strip(start)
                            ));
                        } else {
                            out.push_str(&format!("{command}({}){tail}", latex(&summand)));
                        }
                    }
                    _ => {
                        // `x \to a`, with `a^+` or `a^-` for one side.
                        if let Some((variable, target)) = lower.split_once("->") {
                            let target = target.trim();
                            let (point, side) = if let Some(p) = target
                                .strip_suffix("^(+)")
                                .or_else(|| target.strip_suffix("^+"))
                            {
                                (p, ", +")
                            } else if let Some(p) = target
                                .strip_suffix("^(-)")
                                .or_else(|| target.strip_suffix("^-"))
                            {
                                (p, ", -")
                            } else {
                                (target, "")
                            };
                            out.push_str(&format!(
                                "limit({}, {}, {}{side})",
                                latex(body.trim()),
                                strip(variable),
                                strip(point)
                            ));
                        } else {
                            out.push_str(&format!("limit({})", latex(body.trim())));
                        }
                    }
                }
                return out;
            }
            "begin" => {
                if let Some((environment, after_begin)) = group(&chars, rest) {
                    let end_marker = format!("\\end{{{environment}}}");
                    let tail: String = chars[after_begin..].iter().collect();
                    if let Some(end) = tail.find(&end_marker) {
                        let body = &tail[..end];
                        let rows: Vec<String> = body
                            .split("\\\\")
                            .filter(|r| !r.trim().is_empty())
                            .map(|row| {
                                format!(
                                    "[{}]",
                                    row.split('&')
                                        .map(|cell| convert(cell.trim()))
                                        .collect::<Vec<_>>()
                                        .join(",")
                                )
                            })
                            .collect();
                        out.push_str(&format!("[{}]", rows.join(",")));
                        let consumed = after_begin + tail[..end + end_marker.len()].chars().count();
                        i = consumed;
                        continue;
                    }
                }
            }
            "operatorname" | "mathrm" | "text" | "mathbf" => {
                if let Some((inner, next)) = group(&chars, skip_spaces(&chars, rest)) {
                    out.push_str(&inner);
                    i = next;
                    continue;
                }
            }
            n if function_names().contains(&n) => {
                // `\log_b x`, `\sin^2 x`, `\ln x`, `\det M`.
                let (subscript, after_sub) =
                    script(&chars, rest, '_').unwrap_or((String::new(), rest));
                let (power, after_pow) =
                    script(&chars, after_sub, '^').unwrap_or((String::new(), after_sub));
                let (argument, next) = bare_argument(&chars, after_pow);
                if argument.is_empty() {
                    out.push_str(n);
                    i = rest;
                    continue;
                }
                let call = if n == "log" && !subscript.is_empty() {
                    format!(
                        "log({argument}, {})",
                        subscript.trim_matches(|ch| ch == '(' || ch == ')')
                    )
                } else {
                    format!("{n}({argument})")
                };
                if power.is_empty() {
                    out.push_str(&call);
                } else {
                    out.push_str(&format!(
                        "{call}^({})",
                        power.trim_matches(|ch| ch == '(' || ch == ')')
                    ));
                }
                i = next;
                continue;
            }
            n => {
                if let Some(g) = greek(n) {
                    if g.chars().next().is_some_and(|c| c.is_alphabetic())
                        && out
                            .chars()
                            .last()
                            .is_some_and(|c| c.is_alphanumeric() || c == ')')
                    {
                        out.push('*');
                    }
                    out.push_str(g);
                    // `\pi r^2` is a product.
                    let next = skip_spaces(&chars, rest);
                    if next < chars.len()
                        && (chars[next].is_alphanumeric() || chars[next] == '(')
                        && !g.is_empty()
                        && g != "*"
                        && g != "/"
                        && !g.starts_with(['<', '>', '!', '-', '.'])
                    {
                        out.push('*');
                    }
                    i = rest;
                    continue;
                }
            }
        }
        // Unknown command: keep its name.
        out.push_str(&name);
        i = rest;
    }
    out
}

/// Split `x^2 dx` into `("x^2", "x")`.
fn split_differential(text: &str) -> (String, String) {
    let chars: Vec<char> = text.chars().collect();
    // The last `d<letter>` at the end, preceded by a space, a closing bracket, a digit or an operator.
    if chars.len() >= 2 {
        let last = chars[chars.len() - 1];
        let before = chars[chars.len() - 2];
        if last.is_ascii_alphabetic() && before == 'd' {
            let body: String = chars[..chars.len() - 2].iter().collect();
            let trimmed = body.trim_end();
            let ok = body.is_empty()
                || trimmed.len() < body.len()
                || trimmed.ends_with([')', '}', ']'])
                || trimmed.chars().last().is_some_and(|c| c.is_ascii_digit());
            if ok {
                return (trimmed.to_string(), last.to_string());
            }
        }
    }
    (text.to_string(), "x".to_string())
}

fn number_word(word: &str) -> Option<&'static str> {
    Some(match word {
        "zero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        "ten" => "10",
        _ => return None,
    })
}

/// Rewrite plain English; `None` if the input has no English operators.
pub fn english(input: &str) -> Option<String> {
    if !input.is_ascii() {
        return None;
    }
    // Set expressions (`{1,2,3} minus {3}`) and logic keep their own words.
    if input.contains('{') || input.contains('[') {
        return None;
    }
    let mut text = format!(" {} ", input.trim().trim_end_matches(['?', '=', '.']));
    let original = text.clone();
    // Replacement helpers that ignore case but keep the case of everything else.
    fn replace_ci(text: &str, from: &str, to: &str) -> String {
        let (lower, mut out, mut i) = (text.to_ascii_lowercase(), String::new(), 0);
        while let Some(at) = lower[i..].find(from) {
            out.push_str(&text[i..i + at]);
            out.push_str(to);
            i += at + from.len();
        }
        out.push_str(&text[i..]);
        out
    }
    for prefix in [
        " what is ",
        " what's ",
        " what are ",
        " how much is ",
        " calculate ",
        " compute ",
        " evaluate ",
        " find ",
        " tell me ",
        " please ",
    ] {
        if text.to_ascii_lowercase().starts_with(prefix) {
            text = format!(" {}", &text[prefix.len()..]);
        }
    }
    // Number words.
    let words: Vec<String> = text
        .split_whitespace()
        .map(|w| {
            number_word(&w.to_ascii_lowercase())
                .map(str::to_string)
                .unwrap_or_else(|| w.to_string())
        })
        .collect();
    text = format!(" {} ", words.join(" "));
    let replacements = [
        (" multiplied by ", " * "),
        (" times ", " * "),
        (" divided by ", " / "),
        (" plus ", " + "),
        (" minus ", " - "),
        (" over ", " / "),
        (" mod ", " mod "),
        (" to the power of ", " ^ "),
        (" to the power ", " ^ "),
        (" to the ", " ^ "),
        (" raised to ", " ^ "),
    ];
    for (from, to) in replacements {
        text = replace_ci(&text, from, to);
    }
    // `x squared`, `x cubed`.
    text = replace_ci(&replace_ci(&text, " squared ", "^2 "), " cubed ", "^3 ");
    // `square root of 16`, `cube root of 27`, `factorial of 5`, `absolute value of -3`, `20 percent of 150`.
    let unary = [
        ("square root of ", "sqrt"),
        ("cube root of ", "cbrt"),
        ("absolute value of ", "abs"),
        ("natural log of ", "ln"),
        ("log of ", "log"),
        ("sine of ", "sin"),
        ("cosine of ", "cos"),
        ("tangent of ", "tan"),
        ("exponential of ", "exp"),
    ];
    for (phrase, function) in unary {
        while let Some(at) = text.to_ascii_lowercase().find(phrase) {
            let after = at + phrase.len();
            let token: String = text[after..]
                .chars()
                .take_while(|c| !c.is_whitespace())
                .collect();
            if token.is_empty() {
                break;
            }
            let end = after + token.len();
            text = format!("{}{function}({token}){}", &text[..at], &text[end..]);
        }
    }
    while let Some(at) = text.to_ascii_lowercase().find("factorial of ") {
        let after = at + "factorial of ".len();
        let token: String = text[after..]
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if token.is_empty() {
            break;
        }
        let end = after + token.len();
        text = format!("{}({token})!{}", &text[..at], &text[end..]);
    }
    if let Some(at) = text.to_ascii_lowercase().find(" percent of ") {
        let before: String = text[..at]
            .trim()
            .rsplit(' ')
            .next()
            .unwrap_or("")
            .to_string();
        let after: String = text[at + " percent of ".len()..]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        if !before.is_empty() && !after.is_empty() {
            let start = text[..at].rfind(&before).unwrap_or(0);
            let end = at + " percent of ".len() + after.len();
            text = format!(
                "{}(({before})/100*({after})){}",
                &text[..start],
                &text[end..]
            );
        }
    }
    let result = text.trim().replace(" ^ ", "^");
    if format!(" {} ", result) == original.trim_end_matches(' ').to_string() + " "
        || result.is_empty()
    {
        return None;
    }
    // Only when an English operator was actually used.
    let changed = format!(" {result} ") != original;
    changed.then_some(result)
}

/// The rewritten input, or `None` if there is nothing to rewrite.
pub fn normalise(input: &str) -> Option<String> {
    let original = input.trim();
    if let Some(expanded) = expand_radix_literals(original) {
        return Some(expanded);
    }
    if let Some(expanded) = expand_degrees(original) {
        return Some(expanded);
    }
    if let Some(short) = shorthand(original) {
        return Some(short);
    }
    // Unicode operators become their LaTeX spellings and take the LaTeX route.
    let mapped = original
        .replace('∫', "\\int ")
        .replace('∑', "\\sum")
        .replace('∏', "\\prod")
        .replace('∞', "\\infty ");
    let trimmed = mapped.trim();
    if trimmed.contains('\\') {
        let converted = convert(trimmed);
        return (converted != trimmed).then_some(converted);
    }
    english(trimmed)
}

/// Parentheses around `s` unless it is already a single atom (a number, an identifier, a call or a bracketed group).
fn wrap(s: &str) -> String {
    let simple = (!s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.'))
        || is_single_call(s);
    if simple {
        s.to_string()
    } else {
        format!("({s})")
    }
}

/// `name(...)` or `(...)` whose bracket closes at the very end.
fn is_single_call(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let Some(open) = chars.iter().position(|&c| c == '(') else {
        return false;
    };
    if !chars[..open]
        .iter()
        .all(|c| c.is_alphanumeric() || *c == '_')
    {
        return false;
    }
    let mut depth = 0;
    for (i, &c) in chars.iter().enumerate().skip(open) {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return i == chars.len() - 1;
            }
        }
    }
    false
}

/// Convert LaTeX to the engine's syntax.
pub fn latex(input: &str) -> String {
    let converted = convert(input);
    // One pair of brackets around everything (`a fraction alone`) is not needed.
    if converted.starts_with('(') && is_single_call(&converted) {
        return converted[1..converted.len() - 1].to_string();
    }
    converted
}

/// Split `k^2 - 1` into the summand `k^2` and the tail ` - 1`: a sum, product or limit applies to the next term only,
/// which ends at a `+` or `-` outside brackets.
fn split_scope(body: &str) -> (String, String) {
    let chars: Vec<char> = body.chars().collect();
    let mut depth = 0i32;
    let mut seen = false;
    let mut previous = ' ';
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if depth == 0
            && (c == '+' || c == '-')
            && seen
            && !matches!(previous, '*' | '/' | '^' | '(' | ',' | '=' | '+' | '-')
        {
            return (chars[..i].iter().collect(), chars[i..].iter().collect());
        }
        if !c.is_whitespace() {
            seen = true;
            previous = c;
        }
    }
    (body.to_string(), String::new())
}

// ------------------------------------------------------------------------------------------------------------
// Shorthand a user types without brackets: `d/dx x^3`, `d^2/dx^2 sin(x)`, `lim x->0 sin(x)/x`, `sqrt 16`, `sin x`.
// ------------------------------------------------------------------------------------------------------------

fn digits_exponent(chars: &[char], i: &mut usize) -> Option<u32> {
    match chars.get(*i) {
        Some('²') => {
            *i += 1;
            Some(2)
        }
        Some('³') => {
            *i += 1;
            Some(3)
        }
        Some('^') => {
            let start = *i + 1;
            let mut j = start;
            while chars.get(j).is_some_and(|c| c.is_ascii_digit()) {
                j += 1;
            }
            if j == start {
                return None;
            }
            let n: String = chars[start..j].iter().collect();
            *i = j;
            n.parse().ok()
        }
        _ => Some(1),
    }
}

/// `d/dx f`, `d/dx(f)`, `d^2/dx^2 f`, `∂/∂x f` as `diff(f, x, n)`.
fn derivative_shorthand(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    let is_d = |c: char| c == 'd' || c == '∂';
    let mut i = 0;
    if !is_d(*chars.first()?) {
        return None;
    }
    i += 1;
    let order = digits_exponent(&chars, &mut i)?;
    if chars.get(i) != Some(&'/') {
        return None;
    }
    i += 1;
    if !is_d(*chars.get(i)?) {
        return None;
    }
    i += 1;
    let variable = *chars.get(i)?;
    if !variable.is_alphabetic() {
        return None;
    }
    i += 1;
    let denominator_order = digits_exponent(&chars, &mut i)?;
    if denominator_order != order {
        return None;
    }
    let rest: String = chars[i..].iter().collect();
    let rest = rest.trim();
    if rest.is_empty()
        || rest.starts_with('=')
        || !(chars[i..]
            .first()
            .is_some_and(|c| c.is_whitespace() || *c == '('))
    {
        return None;
    }
    Some(format!("diff({rest}, {variable}, {order})"))
}

/// `lim x->a f` and `limit x -> a f` as `limit(f, x, a)` (a two-sided limit).
fn limit_shorthand(s: &str) -> Option<String> {
    let rest = s.strip_prefix("limit").or_else(|| s.strip_prefix("lim"))?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let arrow = rest
        .find("->")
        .map(|at| (at, 2))
        .or_else(|| rest.find('→').map(|at| (at, '→'.len_utf8())))?;
    let variable = rest[..arrow.0].trim();
    if variable.is_empty() || !variable.chars().all(|c| c.is_alphabetic() || c == '_') {
        return None;
    }
    let after = rest[arrow.0 + arrow.1..].trim_start();
    let end = after.find(char::is_whitespace)?;
    let (target, function) = (&after[..end], after[end..].trim());
    if function.is_empty() || target.ends_with('+') || target.ends_with('-') {
        return None;
    }
    let target = if target == "inf" || target == "infinity" || target == "∞" {
        "oo"
    } else {
        target
    };
    Some(format!("limit({function}, {variable}, {target})"))
}

/// `sqrt 16`, `sin x`: a known function followed by a single bare atom.
fn function_space_shorthand(s: &str) -> Option<String> {
    const NAMES: [&str; 19] = [
        "sqrt", "sin", "cos", "tan", "sec", "csc", "cot", "asin", "acos", "atan", "sinh", "cosh",
        "tanh", "ln", "exp", "abs", "log", "floor", "ceil",
    ];
    let (name, atom) = s.split_once(' ')?;
    let atom = atom.trim();
    if NAMES.contains(&name)
        && !atom.is_empty()
        && atom
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
    {
        return Some(format!("{name}({atom})"));
    }
    None
}

/// Brackets-free calculus shorthand and a few spellings, or `None` when the input is not one of them.
pub fn shorthand(input: &str) -> Option<String> {
    let t = input.trim();
    if let Some(rest) = t.strip_prefix("derivative(") {
        return Some(format!("diff({rest}"));
    }
    if let Some(rest) = t.strip_prefix("nth_root(") {
        return Some(format!("root({rest}"));
    }
    set_shorthand(t)
        .or_else(|| bits_shorthand(t))
        .or_else(|| choose_shorthand(t))
        .or_else(|| bars_shorthand(t))
        .or_else(|| derivative_shorthand(t))
        .or_else(|| limit_shorthand(t))
        .or_else(|| function_space_shorthand(t))
}

/// `10 choose 3` as `choose(10, 3)`.
fn choose_shorthand(s: &str) -> Option<String> {
    let (n, k) = s.split_once(" choose ")?;
    let atom = |t: &str| {
        let t = t.trim();
        !t.is_empty()
            && t.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    };
    (atom(n) && atom(k)).then(|| format!("choose({}, {})", n.trim(), k.trim()))
}

/// `|x - 3|` as `abs(x - 3)` (one pair of bars around the whole input).
fn bars_shorthand(s: &str) -> Option<String> {
    let inner = s.strip_prefix('|')?.strip_suffix('|')?;
    (!inner.is_empty() && !inner.contains('|')).then(|| format!("abs({inner})"))
}

// ------------------------------------------------------------------------------------------------------------
// Number bases and bit operations typed as text: `0xFF`, `0b1010 + 1`, `255 to binary`, `5 xor 3`, `6 << 2`.
// ------------------------------------------------------------------------------------------------------------

/// `0xFF`, `0b1010`, `0o17` replaced by their decimal values; `None` when the text has no such literal.
pub fn expand_radix_literals(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    let (mut out, mut i, mut changed) = (String::new(), 0usize, false);
    while i < chars.len() {
        let at_start = i == 0
            || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_' || chars[i - 1] == '.');
        if at_start
            && chars[i] == '0'
            && let Some(&marker) = chars.get(i + 1)
        {
            let base = match marker {
                'x' | 'X' => 16,
                'b' | 'B' => 2,
                'o' | 'O' => 8,
                _ => 0,
            };
            if base != 0 {
                let start = i + 2;
                let mut end = start;
                while end < chars.len() && chars[end].is_digit(base) {
                    end += 1;
                }
                let boundary = chars
                    .get(end)
                    .is_none_or(|c| !(c.is_alphanumeric() || *c == '_'));
                if end > start && boundary {
                    let digits: String = chars[start..end].iter().collect();
                    if let Some(value) = num_bigint::BigInt::parse_bytes(digits.as_bytes(), base) {
                        out.push_str(&value.to_string());
                        i = end;
                        changed = true;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    changed.then_some(out)
}

/// `255 to binary` as `dec2bin(255)` and `5 xor 3`, `5 & 3`, `5 | 3`, `6 << 2`, `6 >> 1` as `bitxor(5, 3)`, ...
fn bits_shorthand(s: &str) -> Option<String> {
    let t = s.trim();
    let integer = |x: &str| {
        let x = x.trim();
        !x.is_empty() && x.chars().all(|c| c.is_ascii_digit())
    };
    for (word, command) in [
        ("binary", "dec2bin"),
        ("hexadecimal", "dec2hex"),
        ("hex", "dec2hex"),
        ("octal", "dec2oct"),
    ] {
        for connective in [" to ", " in ", " as "] {
            if let Some((n, target)) = t.split_once(connective)
                && integer(n)
                && target.trim().eq_ignore_ascii_case(word)
            {
                return Some(format!("{command}({})", n.trim()));
            }
        }
    }
    for (op, command) in [
        (" xor ", "bitxor"),
        ("<<", "shl"),
        (">>", "shr"),
        ("&", "bitand"),
        ("|", "bitor"),
    ] {
        if let Some((a, b)) = t.split_once(op)
            && integer(a)
            && integer(b)
        {
            return Some(format!("{command}({}, {})", a.trim(), b.trim()));
        }
    }
    None
}

/// `|{1,2,3}|` and `cardinality({1,2,3})` as `size {1,2,3}`; `cartesian_product(A, B)` and `A x B` as `A cross B`.
fn set_shorthand(s: &str) -> Option<String> {
    let t = s.trim();
    if let Some(inner) = t.strip_prefix('|').and_then(|r| r.strip_suffix('|'))
        && inner.trim().starts_with('{')
        && inner.trim().ends_with('}')
    {
        return Some(format!("size {}", inner.trim()));
    }
    for name in ["cardinality(", "card(", "size_of("] {
        if let Some(inner) = t.strip_prefix(name).and_then(|r| r.strip_suffix(')'))
            && inner.trim().starts_with('{')
        {
            return Some(format!("size {}", inner.trim()));
        }
    }
    if let Some(inner) = t
        .strip_prefix("cartesian_product(")
        .or_else(|| t.strip_prefix("cross_product_sets("))
        .and_then(|r| r.strip_suffix(')'))
    {
        let (mut depth, mut at) = (0i32, None);
        for (i, c) in inner.char_indices() {
            match c {
                '{' | '(' | '[' => depth += 1,
                '}' | ')' | ']' => depth -= 1,
                ',' if depth == 0 && at.is_none() => at = Some(i),
                _ => {}
            }
        }
        let at = at?;
        return Some(format!(
            "{} cross {}",
            inner[..at].trim(),
            inner[at + 1..].trim()
        ));
    }
    if t.starts_with('{') && t.ends_with('}') {
        for times in ["} x {", "} × {", "}×{", "} X {"] {
            if t.matches(times).count() == 1 {
                return Some(t.replacen(times, "} cross {", 1));
            }
        }
    }
    None
}

/// `30°`, `30 deg`, `60 degrees`, `2.5 degree` replaced by `(30*pi/180)`; `None` when the text has none.
pub fn expand_degrees(s: &str) -> Option<String> {
    // `convert 180 deg to rad` names the unit degree: leave unit conversions alone.
    let lowered = s.to_ascii_lowercase();
    if lowered.starts_with("convert ")
        || [" to ", " in ", " as "].iter().any(|w| lowered.contains(w))
    {
        return None;
    }
    let chars: Vec<char> = s.chars().collect();
    let (mut out, mut i, mut changed) = (String::new(), 0usize, false);
    while i < chars.len() {
        let starts_number = chars[i].is_ascii_digit()
            && (i == 0
                || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_' || chars[i - 1] == '.'));
        if starts_number {
            let mut end = i;
            while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.') {
                end += 1;
            }
            let mut after = end;
            while after < chars.len() && chars[after] == ' ' {
                after += 1;
            }
            let unit_len = if chars.get(after) == Some(&'°') {
                Some(1)
            } else {
                ["degrees", "degree", "deg"].iter().find_map(|word| {
                    let w: Vec<char> = word.chars().collect();
                    let boundary = chars
                        .get(after + w.len())
                        .is_none_or(|c| !(c.is_alphanumeric() || *c == '_'));
                    (chars[after.min(chars.len())..].starts_with(&w) && boundary).then_some(w.len())
                })
            };
            if let Some(len) = unit_len {
                let number: String = chars[i..end].iter().collect();
                out.push_str(&format!("({number}*π/180)"));
                i = after + len;
                changed = true;
                continue;
            }
            let number: String = chars[i..end].iter().collect();
            out.push_str(&number);
            i = end;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    changed.then_some(out)
}
