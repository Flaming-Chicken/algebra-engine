//! # `algebra-logic`
//!
//! Boolean logic, 3-valued logics (Kleene K3, Łukasiewicz Ł3, Bochvar), constructible $N$-valued logic truth tables,
//! predicate calculus, assumptions engine, and SAT solver for URAE.

use algebra_core::SymbolId;
use std::collections::{HashMap, HashSet};

/// Truth values in 3-valued logic systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ThreeValuedValue {
    /// True value (1)
    True = 1,
    /// False value (0)
    False = 0,
    /// Unknown / Undefined / Middle value (u / m)
    Unknown = 2,
}

/// Supported 3-valued logic systems from literature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ThreeValuedSystem {
    /// Kleene's Strong 3-Valued Logic (K3)
    #[default]
    KleeneK3,
    /// Łukasiewicz 3-Valued Logic (Ł3)
    LukasiewiczL3,
    /// Bochvar 3-Valued Logic (Meaningless/Nonsense propagation)
    Bochvar,
    /// Gödel-Dummett 3-Valued Intuitionistic Logic (G3)
    GodelG3,
}

/// Evaluator for 3-valued logic operations.
pub struct ThreeValuedLogic;

impl ThreeValuedLogic {
    /// Evaluate 3-valued NOT.
    pub fn not(a: ThreeValuedValue, system: ThreeValuedSystem) -> ThreeValuedValue {
        match system {
            ThreeValuedSystem::GodelG3 => match a {
                ThreeValuedValue::False => ThreeValuedValue::True,
                _ => ThreeValuedValue::False, // ¬u = 0 in Gödel intuitionistic logic
            },
            _ => match a {
                ThreeValuedValue::True => ThreeValuedValue::False,
                ThreeValuedValue::False => ThreeValuedValue::True,
                ThreeValuedValue::Unknown => ThreeValuedValue::Unknown,
            },
        }
    }

    /// Evaluate 3-valued AND.
    pub fn and(
        a: ThreeValuedValue,
        b: ThreeValuedValue,
        system: ThreeValuedSystem,
    ) -> ThreeValuedValue {
        match system {
            ThreeValuedSystem::Bochvar => {
                if a == ThreeValuedValue::Unknown || b == ThreeValuedValue::Unknown {
                    ThreeValuedValue::Unknown
                } else if a == ThreeValuedValue::True && b == ThreeValuedValue::True {
                    ThreeValuedValue::True
                } else {
                    ThreeValuedValue::False
                }
            }
            ThreeValuedSystem::KleeneK3
            | ThreeValuedSystem::LukasiewiczL3
            | ThreeValuedSystem::GodelG3 => {
                if a == ThreeValuedValue::False || b == ThreeValuedValue::False {
                    ThreeValuedValue::False
                } else if a == ThreeValuedValue::True && b == ThreeValuedValue::True {
                    ThreeValuedValue::True
                } else {
                    ThreeValuedValue::Unknown
                }
            }
        }
    }

    /// Evaluate 3-valued OR.
    pub fn or(
        a: ThreeValuedValue,
        b: ThreeValuedValue,
        system: ThreeValuedSystem,
    ) -> ThreeValuedValue {
        match system {
            ThreeValuedSystem::Bochvar => {
                if a == ThreeValuedValue::Unknown || b == ThreeValuedValue::Unknown {
                    ThreeValuedValue::Unknown
                } else if a == ThreeValuedValue::True || b == ThreeValuedValue::True {
                    ThreeValuedValue::True
                } else {
                    ThreeValuedValue::False
                }
            }
            ThreeValuedSystem::KleeneK3
            | ThreeValuedSystem::LukasiewiczL3
            | ThreeValuedSystem::GodelG3 => {
                if a == ThreeValuedValue::True || b == ThreeValuedValue::True {
                    ThreeValuedValue::True
                } else if a == ThreeValuedValue::False && b == ThreeValuedValue::False {
                    ThreeValuedValue::False
                } else {
                    ThreeValuedValue::Unknown
                }
            }
        }
    }

    /// Evaluate 3-valued Implication (A -> B).
    pub fn implies(
        a: ThreeValuedValue,
        b: ThreeValuedValue,
        system: ThreeValuedSystem,
    ) -> ThreeValuedValue {
        match system {
            ThreeValuedSystem::LukasiewiczL3 => {
                if a == ThreeValuedValue::Unknown && b == ThreeValuedValue::Unknown {
                    ThreeValuedValue::True // Ł3 gives u -> u = True
                } else if a == ThreeValuedValue::Unknown && b == ThreeValuedValue::False {
                    ThreeValuedValue::Unknown
                } else {
                    Self::or(Self::not(a, system), b, system)
                }
            }
            ThreeValuedSystem::GodelG3 => {
                // Gödel implication: a <= b => 1, else b
                if a == b || a == ThreeValuedValue::False || b == ThreeValuedValue::True {
                    ThreeValuedValue::True
                } else {
                    b
                }
            }
            _ => Self::or(Self::not(a, system), b, system),
        }
    }

    /// Modal Necessity Operator: $\Box A$ (true only if definitely True).
    pub fn necessity(a: ThreeValuedValue) -> ThreeValuedValue {
        match a {
            ThreeValuedValue::True => ThreeValuedValue::True,
            _ => ThreeValuedValue::False,
        }
    }

    /// Modal Possibility Operator: $\Diamond A = \neg \Box \neg A$ (true if not definitely False).
    pub fn possibility(a: ThreeValuedValue) -> ThreeValuedValue {
        match a {
            ThreeValuedValue::False => ThreeValuedValue::False,
            _ => ThreeValuedValue::True,
        }
    }
}

/// Custom $N$-Valued Logic Truth Table Builder.
#[derive(Debug, Clone)]
pub struct GeneralizedValuedLogic<const N: usize> {
    pub designated_values: HashSet<u8>,
    pub not_table: [u8; N],
    pub and_table: [[u8; N]; N],
    pub or_table: [[u8; N]; N],
}

impl<const N: usize> GeneralizedValuedLogic<N> {
    /// Create a default $N$-valued logic structure.
    pub fn new(designated: &[u8]) -> Self {
        let mut designated_values = HashSet::new();
        for &d in designated {
            designated_values.insert(d);
        }

        let mut not_table = [0u8; N];
        for (i, slot) in not_table.iter_mut().enumerate() {
            *slot = (N - 1 - i) as u8;
        }

        let mut and_table = [[0u8; N]; N];
        let mut or_table = [[0u8; N]; N];
        for i in 0..N {
            for j in 0..N {
                and_table[i][j] = (i.min(j)) as u8;
                or_table[i][j] = (i.max(j)) as u8;
            }
        }

        Self {
            designated_values,
            not_table,
            and_table,
            or_table,
        }
    }

    /// Evaluate negation.
    pub fn eval_not(&self, val: u8) -> u8 {
        let idx = (val as usize).min(N - 1);
        self.not_table[idx]
    }

    /// Evaluate conjunction.
    pub fn eval_and(&self, a: u8, b: u8) -> u8 {
        let ia = (a as usize).min(N - 1);
        let ib = (b as usize).min(N - 1);
        self.and_table[ia][ib]
    }

    /// Evaluate disjunction.
    pub fn eval_or(&self, a: u8, b: u8) -> u8 {
        let ia = (a as usize).min(N - 1);
        let ib = (b as usize).min(N - 1);
        self.or_table[ia][ib]
    }

    /// Check if a truth value belongs to designated true values.
    pub fn is_designated(&self, val: u8) -> bool {
        self.designated_values.contains(&val)
    }
}

/// Logical mathematical predicates for assumptions on variable symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Predicate {
    /// Number is real.
    Real,
    /// Number is strictly positive (> 0).
    Positive,
    /// Number is non-negative (>= 0).
    NonNegative,
    /// Number is strictly negative (< 0).
    Negative,
    /// Number is non-zero (!= 0).
    NonZero,
    /// Number is an integer.
    Integer,
    /// Number is even integer.
    Even,
    /// Number is odd integer.
    Odd,
    /// Number is prime.
    Prime,
    /// Operation is commutative.
    Commutative,
}

/// Interval bound for real variables $x \in [min, max]$ or open intervals.
#[derive(Debug, Clone, PartialEq)]
pub struct IntervalBound {
    pub min_value: Option<f64>,
    pub min_inclusive: bool,
    pub max_value: Option<f64>,
    pub max_inclusive: bool,
}

impl IntervalBound {
    pub fn closed(min: f64, max: f64) -> Self {
        Self {
            min_value: Some(min),
            min_inclusive: true,
            max_value: Some(max),
            max_inclusive: true,
        }
    }

    pub fn open(min: f64, max: f64) -> Self {
        Self {
            min_value: Some(min),
            min_inclusive: false,
            max_value: Some(max),
            max_inclusive: false,
        }
    }

    pub fn positive() -> Self {
        Self {
            min_value: Some(0.0),
            min_inclusive: false,
            max_value: None,
            max_inclusive: false,
        }
    }

    /// Check if value `val` lies within interval.
    pub fn contains(&self, val: f64) -> bool {
        if let Some(min) = self.min_value {
            if self.min_inclusive {
                if val < min {
                    return false;
                }
            } else if val <= min {
                return false;
            }
        }
        if let Some(max) = self.max_value {
            if self.max_inclusive {
                if val > max {
                    return false;
                }
            } else if val >= max {
                return false;
            }
        }
        true
    }
}

/// Complex branch cut specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BranchCut {
    /// Logarithmic branch cut along negative real axis $(-\infty, 0]$
    NegativeRealAxis,
    /// Square root branch cut along negative real axis $(-\infty, 0)$
    StrictNegativeRealAxis,
}

/// Context tracking assumptions attached to symbols.
#[derive(Debug, Clone, Default)]
pub struct AssumptionsContext {
    assumptions: HashMap<SymbolId, HashSet<Predicate>>,
    bounds: HashMap<SymbolId, IntervalBound>,
    branch_cuts: HashMap<SymbolId, Vec<BranchCut>>,
}

impl AssumptionsContext {
    /// Create a new empty assumptions context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach interval bounds to a symbol.
    pub fn set_bounds(&mut self, symbol: SymbolId, bounds: IntervalBound) {
        self.bounds.insert(symbol, bounds);
    }

    /// Get interval bounds for a symbol if set.
    pub fn get_bounds(&self, symbol: SymbolId) -> Option<&IntervalBound> {
        self.bounds.get(&symbol)
    }

    /// Add a branch cut tracking constraint for a symbol.
    pub fn add_branch_cut(&mut self, symbol: SymbolId, cut: BranchCut) {
        self.branch_cuts.entry(symbol).or_default().push(cut);
    }

    /// Get active branch cuts for a symbol.
    pub fn get_branch_cuts(&self, symbol: SymbolId) -> Option<&[BranchCut]> {
        self.branch_cuts.get(&symbol).map(|v| v.as_slice())
    }

    /// Add an assumption predicate for a symbol.
    pub fn assume(&mut self, symbol: SymbolId, predicate: Predicate) {
        let set = self.assumptions.entry(symbol).or_default();
        set.insert(predicate);

        match predicate {
            Predicate::Positive => {
                set.insert(Predicate::NonNegative);
                set.insert(Predicate::NonZero);
                set.insert(Predicate::Real);
            }
            Predicate::Negative => {
                set.insert(Predicate::NonZero);
                set.insert(Predicate::Real);
            }
            Predicate::NonNegative => {
                set.insert(Predicate::Real);
            }
            Predicate::Even | Predicate::Odd | Predicate::Prime => {
                set.insert(Predicate::Integer);
                set.insert(Predicate::Real);
            }
            Predicate::Integer => {
                set.insert(Predicate::Real);
            }
            _ => {}
        }
    }

    /// Check if a symbol satisfies a predicate based on current assumptions.
    pub fn is(&self, symbol: SymbolId, predicate: Predicate) -> bool {
        if let Some(set) = self.assumptions.get(&symbol) {
            set.contains(&predicate)
        } else {
            false
        }
    }

    /// Check for contradictions in current symbol assumptions.
    pub fn is_consistent(&self) -> bool {
        for set in self.assumptions.values() {
            if set.contains(&Predicate::Positive) && set.contains(&Predicate::Negative) {
                return false;
            }
            if set.contains(&Predicate::Even) && set.contains(&Predicate::Odd) {
                return false;
            }
        }
        true
    }
}

/// AST for propositional boolean logic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BoolExpr {
    /// Boolean constant true.
    True,
    /// Boolean constant false.
    False,
    /// Proposition variable.
    Var(u32),
    /// Logical NOT.
    Not(Box<BoolExpr>),
    /// Logical AND.
    And(Vec<BoolExpr>),
    /// Logical OR.
    Or(Vec<BoolExpr>),
    /// Logical implication A => B.
    Implies(Box<BoolExpr>, Box<BoolExpr>),
}

impl BoolExpr {
    /// Simplify boolean expression using truth identities.
    pub fn simplify(&self) -> BoolExpr {
        match self {
            BoolExpr::Not(inner) => match inner.simplify() {
                BoolExpr::True => BoolExpr::False,
                BoolExpr::False => BoolExpr::True,
                BoolExpr::Not(x) => *x,
                other => BoolExpr::Not(Box::new(other)),
            },
            BoolExpr::And(terms) => {
                let mut simplified = Vec::new();
                for t in terms {
                    match t.simplify() {
                        BoolExpr::False => return BoolExpr::False,
                        BoolExpr::True => continue,
                        s => simplified.push(s),
                    }
                }
                if simplified.is_empty() {
                    BoolExpr::True
                } else if simplified.len() == 1 {
                    simplified.remove(0)
                } else {
                    BoolExpr::And(simplified)
                }
            }
            BoolExpr::Or(terms) => {
                let mut simplified = Vec::new();
                for t in terms {
                    match t.simplify() {
                        BoolExpr::True => return BoolExpr::True,
                        BoolExpr::False => continue,
                        s => simplified.push(s),
                    }
                }
                if simplified.is_empty() {
                    BoolExpr::False
                } else if simplified.len() == 1 {
                    simplified.remove(0)
                } else {
                    BoolExpr::Or(simplified)
                }
            }
            BoolExpr::Implies(a, b) => {
                let sa = a.simplify();
                let sb = b.simplify();
                match (&sa, &sb) {
                    (BoolExpr::False, _) | (_, BoolExpr::True) => BoolExpr::True,
                    (BoolExpr::True, x) => x.clone(),
                    _ => BoolExpr::Or(vec![BoolExpr::Not(Box::new(sa)), sb]),
                }
            }
            _ => self.clone(),
        }
    }
}

/// DPLL SAT Solver & Truth Table generator for propositional logic.
pub struct SatSolver;

impl BoolExpr {
    /// Collect all variable IDs referenced in the formula.
    pub fn collect_vars(&self, out: &mut std::collections::BTreeSet<u32>) {
        match self {
            BoolExpr::True | BoolExpr::False => {}
            BoolExpr::Var(v) => {
                out.insert(*v);
            }
            BoolExpr::Not(inner) => inner.collect_vars(out),
            BoolExpr::And(terms) | BoolExpr::Or(terms) => {
                for t in terms {
                    t.collect_vars(out);
                }
            }
            BoolExpr::Implies(a, b) => {
                a.collect_vars(out);
                b.collect_vars(out);
            }
        }
    }

    /// Substitute `var = value` into the formula without simplifying away structure yet.
    pub fn substitute_var(&self, var: u32, value: bool) -> BoolExpr {
        match self {
            BoolExpr::True => BoolExpr::True,
            BoolExpr::False => BoolExpr::False,
            BoolExpr::Var(v) => {
                if *v == var {
                    if value {
                        BoolExpr::True
                    } else {
                        BoolExpr::False
                    }
                } else {
                    BoolExpr::Var(*v)
                }
            }
            BoolExpr::Not(inner) => BoolExpr::Not(Box::new(inner.substitute_var(var, value))),
            BoolExpr::And(terms) => {
                BoolExpr::And(terms.iter().map(|t| t.substitute_var(var, value)).collect())
            }
            BoolExpr::Or(terms) => {
                BoolExpr::Or(terms.iter().map(|t| t.substitute_var(var, value)).collect())
            }
            BoolExpr::Implies(a, b) => BoolExpr::Implies(
                Box::new(a.substitute_var(var, value)),
                Box::new(b.substitute_var(var, value)),
            ),
        }
    }

    /// Evaluate a closed or partially-bound boolean expression under an assignment map.
    pub fn eval_with(&self, env: &HashMap<u32, bool>) -> Option<bool> {
        match self {
            BoolExpr::True => Some(true),
            BoolExpr::False => Some(false),
            BoolExpr::Var(v) => env.get(v).copied(),
            BoolExpr::Not(inner) => inner.eval_with(env).map(|b| !b),
            BoolExpr::And(terms) => {
                let mut all_known = true;
                for t in terms {
                    match t.eval_with(env) {
                        Some(false) => return Some(false),
                        None => all_known = false,
                        Some(true) => {}
                    }
                }
                if all_known { Some(true) } else { None }
            }
            BoolExpr::Or(terms) => {
                let mut all_known = true;
                for t in terms {
                    match t.eval_with(env) {
                        Some(true) => return Some(true),
                        None => all_known = false,
                        Some(false) => {}
                    }
                }
                if all_known { Some(false) } else { None }
            }
            BoolExpr::Implies(a, b) => match (a.eval_with(env), b.eval_with(env)) {
                (Some(false), _) | (_, Some(true)) => Some(true),
                (Some(true), Some(false)) => Some(false),
                _ => None,
            },
        }
    }
}

impl SatSolver {
    /// Check if a boolean expression is satisfiable via recursive unit-propagating DPLL.
    pub fn is_satisfiable(expr: &BoolExpr) -> bool {
        Self::solve_model(expr).is_some()
    }

    /// Solve a boolean expression and return a satisfying assignment if one exists.
    pub fn solve_model(expr: &BoolExpr) -> Option<std::collections::BTreeMap<u32, bool>> {
        let mut assignment = std::collections::BTreeMap::new();
        Self::dpll_rec(expr.simplify(), &mut assignment).then_some(assignment)
    }

    fn dpll_rec(
        expr: BoolExpr,
        assignment: &mut std::collections::BTreeMap<u32, bool>,
    ) -> bool {
        let simp = expr.simplify();
        match &simp {
            BoolExpr::True => return true,
            BoolExpr::False => return false,
            BoolExpr::Var(v) => {
                assignment.insert(*v, true);
                return true;
            }
            BoolExpr::Not(inner) => {
                if let BoolExpr::Var(v) = inner.as_ref() {
                    assignment.insert(*v, false);
                    return true;
                }
            }
            BoolExpr::And(terms) => {
                // Unit propagation on top-level conjunction literals.
                for t in terms {
                    match t {
                        BoolExpr::Var(v) => {
                            let next = simp.substitute_var(*v, true);
                            let mut branch = assignment.clone();
                            branch.insert(*v, true);
                            if Self::dpll_rec(next, &mut branch) {
                                *assignment = branch;
                                return true;
                            }
                            return false;
                        }
                        BoolExpr::Not(inner) => {
                            if let BoolExpr::Var(v) = inner.as_ref() {
                                let next = simp.substitute_var(*v, false);
                                let mut branch = assignment.clone();
                                branch.insert(*v, false);
                                if Self::dpll_rec(next, &mut branch) {
                                    *assignment = branch;
                                    return true;
                                }
                                return false;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        let mut vars = std::collections::BTreeSet::new();
        simp.collect_vars(&mut vars);
        let Some(&branch_var) = vars.iter().next() else {
            return matches!(simp, BoolExpr::True);
        };

        for val in [true, false] {
            let mut branch_assign = assignment.clone();
            branch_assign.insert(branch_var, val);
            let next = simp.substitute_var(branch_var, val);
            if Self::dpll_rec(next, &mut branch_assign) {
                *assignment = branch_assign;
                return true;
            }
        }
        false
    }

    /// Parse a propositional boolean formula string into `(BoolExpr, ordered_vars)`.
    ///
    /// Supports `not`/`!`/`~`, `and`/`&&`/`&`/`/\`, `or`/`||`/`|`/`\/`, `xor`,
    /// `->`/`=>`/`implies`, `<->`/`<=>`/`iff`, `true`/`false`, parentheses, and identifiers.
    pub fn parse_formula(input: &str) -> Result<(BoolExpr, Vec<(u32, String)>), String> {
        let tokens = tokenize_bool(input)?;
        if tokens.is_empty() {
            return Err("Empty boolean formula".to_string());
        }
        let mut parser = BoolParser {
            tokens,
            pos: 0,
            vars: Vec::new(),
        };
        let expr = parser.parse_iff()?;
        if parser.pos < parser.tokens.len() {
            return Err(format!(
                "Unexpected token near {:?}",
                parser.tokens[parser.pos]
            ));
        }
        Ok((expr, parser.vars))
    }

    /// Generate an ASCII truth table for a propositional logic formula.
    pub fn truth_table(formula: &str) -> Result<String, String> {
        let (expr, vars) = Self::parse_formula(formula)?;
        if vars.len() > 10 {
            return Err(format!(
                "Truth table supports at most 10 variables (got {})",
                vars.len()
            ));
        }
        let n = vars.len();
        let rows = 1usize << n;
        let mut lines = Vec::with_capacity(rows + 2);

        let mut header_cols: Vec<String> = vars.iter().map(|(_, name)| name.clone()).collect();
        header_cols.push(formula.trim().to_string());
        lines.push(header_cols.join(" | "));
        lines.push(
            header_cols
                .iter()
                .map(|c| "-".repeat(c.len().max(1)))
                .collect::<Vec<_>>()
                .join("-+-"),
        );

        let mut env = HashMap::with_capacity(n);
        for mask in 0..rows {
            env.clear();
            let mut row_cols = Vec::with_capacity(n + 1);
            for (idx, (vid, _)) in vars.iter().enumerate() {
                let bit = ((mask >> (n - 1 - idx)) & 1) == 1;
                env.insert(*vid, bit);
                row_cols.push(if bit { "T" } else { "F" }.to_string());
            }
            let out = expr
                .eval_with(&env)
                .ok_or_else(|| "Unbound variable during truth table evaluation".to_string())?;
            row_cols.push(if out { "T" } else { "F" }.to_string());
            lines.push(row_cols.join(" | "));
        }
        Ok(lines.join("\n"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BoolTok {
    LParen,
    RParen,
    Not,
    And,
    Or,
    Xor,
    Implies,
    Iff,
    True,
    False,
    Ident(String),
}

fn tokenize_bool(input: &str) -> Result<Vec<BoolTok>, String> {
    let mut toks = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => {
                toks.push(BoolTok::LParen);
                i += 1;
            }
            ')' => {
                toks.push(BoolTok::RParen);
                i += 1;
            }
            '!' | '~' | '¬' => {
                toks.push(BoolTok::Not);
                i += 1;
            }
            '∧' => {
                toks.push(BoolTok::And);
                i += 1;
            }
            '∨' => {
                toks.push(BoolTok::Or);
                i += 1;
            }
            '⊕' => {
                toks.push(BoolTok::Xor);
                i += 1;
            }
            '→' => {
                toks.push(BoolTok::Implies);
                i += 1;
            }
            '↔' => {
                toks.push(BoolTok::Iff);
                i += 1;
            }
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    i += 2;
                } else {
                    i += 1;
                }
                toks.push(BoolTok::And);
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    i += 2;
                } else {
                    i += 1;
                }
                toks.push(BoolTok::Or);
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '\\' => {
                toks.push(BoolTok::And);
                i += 2;
            }
            '\\' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                toks.push(BoolTok::Or);
                i += 2;
            }
            '=' if i + 1 < chars.len() && chars[i + 1] == '>' => {
                toks.push(BoolTok::Implies);
                i += 2;
            }
            '-' if i + 1 < chars.len() && chars[i + 1] == '>' => {
                toks.push(BoolTok::Implies);
                i += 2;
            }
            '<' if i + 2 < chars.len()
                && (chars[i + 1] == '=' || chars[i + 1] == '-')
                && chars[i + 2] == '>' =>
            {
                toks.push(BoolTok::Iff);
                i += 3;
            }
            '^' => {
                toks.push(BoolTok::Xor);
                i += 1;
            }
            _ if c.is_ascii_alphanumeric() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match word.to_ascii_lowercase().as_str() {
                    "not" => toks.push(BoolTok::Not),
                    "and" => toks.push(BoolTok::And),
                    "or" => toks.push(BoolTok::Or),
                    "xor" => toks.push(BoolTok::Xor),
                    "implies" => toks.push(BoolTok::Implies),
                    "iff" => toks.push(BoolTok::Iff),
                    "true" | "1" => toks.push(BoolTok::True),
                    "false" | "0" => toks.push(BoolTok::False),
                    _ => toks.push(BoolTok::Ident(word)),
                }
            }
            _ => {
                return Err(format!("Unexpected character '{}' in boolean formula", c));
            }
        }
    }
    Ok(toks)
}

struct BoolParser {
    tokens: Vec<BoolTok>,
    pos: usize,
    vars: Vec<(u32, String)>,
}

impl BoolParser {
    fn peek(&self) -> Option<&BoolTok> {
        self.tokens.get(self.pos)
    }

    fn intern_var(&mut self, name: String) -> u32 {
        if let Some((id, _)) = self.vars.iter().find(|(_, n)| *n == name) {
            return *id;
        }
        let id = self.vars.len() as u32;
        self.vars.push((id, name));
        id
    }

    fn parse_iff(&mut self) -> Result<BoolExpr, String> {
        let mut lhs = self.parse_implies()?;
        while matches!(self.peek(), Some(BoolTok::Iff)) {
            self.pos += 1;
            let rhs = self.parse_implies()?;
            // (a -> b) & (b -> a)
            lhs = BoolExpr::And(vec![
                BoolExpr::Implies(Box::new(lhs.clone()), Box::new(rhs.clone())),
                BoolExpr::Implies(Box::new(rhs), Box::new(lhs)),
            ]);
        }
        Ok(lhs)
    }

    fn parse_implies(&mut self) -> Result<BoolExpr, String> {
        let lhs = self.parse_or()?;
        if matches!(self.peek(), Some(BoolTok::Implies)) {
            self.pos += 1;
            let rhs = self.parse_implies()?;
            return Ok(BoolExpr::Implies(Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn parse_or(&mut self) -> Result<BoolExpr, String> {
        let mut terms = vec![self.parse_xor()?];
        while matches!(self.peek(), Some(BoolTok::Or)) {
            self.pos += 1;
            terms.push(self.parse_xor()?);
        }
        if terms.len() == 1 {
            Ok(terms.remove(0))
        } else {
            Ok(BoolExpr::Or(terms))
        }
    }

    fn parse_xor(&mut self) -> Result<BoolExpr, String> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek(), Some(BoolTok::Xor)) {
            self.pos += 1;
            let rhs = self.parse_and()?;
            // (a & !b) | (!a & b)
            lhs = BoolExpr::Or(vec![
                BoolExpr::And(vec![lhs.clone(), BoolExpr::Not(Box::new(rhs.clone()))]),
                BoolExpr::And(vec![BoolExpr::Not(Box::new(lhs)), rhs]),
            ]);
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<BoolExpr, String> {
        let mut terms = vec![self.parse_not()?];
        while matches!(self.peek(), Some(BoolTok::And)) {
            self.pos += 1;
            terms.push(self.parse_not()?);
        }
        if terms.len() == 1 {
            Ok(terms.remove(0))
        } else {
            Ok(BoolExpr::And(terms))
        }
    }

    fn parse_not(&mut self) -> Result<BoolExpr, String> {
        if matches!(self.peek(), Some(BoolTok::Not)) {
            self.pos += 1;
            return Ok(BoolExpr::Not(Box::new(self.parse_not()?)));
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<BoolExpr, String> {
        match self.tokens.get(self.pos).cloned() {
            Some(BoolTok::True) => {
                self.pos += 1;
                Ok(BoolExpr::True)
            }
            Some(BoolTok::False) => {
                self.pos += 1;
                Ok(BoolExpr::False)
            }
            Some(BoolTok::Ident(name)) => {
                self.pos += 1;
                Ok(BoolExpr::Var(self.intern_var(name)))
            }
            Some(BoolTok::LParen) => {
                self.pos += 1;
                let inner = self.parse_iff()?;
                if !matches!(self.peek(), Some(BoolTok::RParen)) {
                    return Err("Expected closing ')' in boolean formula".to_string());
                }
                self.pos += 1;
                Ok(inner)
            }
            other => Err(format!("Unexpected token {:?} in boolean formula", other)),
        }
    }
}
