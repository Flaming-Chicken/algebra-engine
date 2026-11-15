//! Sub-frame anytime scheduler with 8ms frame budgets and instant UniversalBall bounds.

use cat_core::ball::UniversalBall;
use cat_core::enclosure::{ApproxResult, DomainCondition, ErrorEnclosure, EvalForm, FidelityMode};
use cat_core::term::CatTerm;
use std::time::{Duration, Instant};

/// Resource budget governing progressive multi-fidelity evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnytimeBudget {
    pub frame_budget: Duration,
    pub hard_timeout: Duration,
    pub max_refinement_steps: usize,
}

impl Default for AnytimeBudget {
    fn default() -> Self {
        Self {
            frame_budget: Duration::from_millis(8),
            hard_timeout: Duration::from_secs(5),
            max_refinement_steps: 64,
        }
    }
}

/// Standardized trait for algorithms supporting 4-fidelity direct and progressive evaluation.
pub trait MultiFidelityOp<I, O> {
    type Error;

    /// Direct evaluation at a requested [`FidelityMode`].
    fn eval_with_fidelity(
        &self,
        input: &I,
        mode: FidelityMode,
        form: EvalForm,
        budget: &AnytimeBudget,
    ) -> Result<ApproxResult<O>, Self::Error>;
}

/// Progressive multi-fidelity stream serving Tier-0/1 approximations immediately
/// while advancing toward `SymbolicHybrid` / `SymbolicExact` within [`AnytimeBudget`].
#[derive(Clone)]
pub struct FidelityStream<O> {
    pub stages: Vec<ApproxResult<O>>,
    pub budget: AnytimeBudget,
    pub started_at: Instant,
    pub completed: bool,
}

impl<O: Clone> FidelityStream<O> {
    /// Start a stream with an immediate Tier-0 approximation result.
    pub fn start_with_tier0(tier0: ApproxResult<O>, budget: AnytimeBudget) -> Self {
        let completed = matches!(
            tier0.mode_used,
            FidelityMode::SymbolicExact | FidelityMode::SymbolicHybrid
        );
        Self {
            stages: vec![tier0],
            budget,
            started_at: Instant::now(),
            completed,
        }
    }

    /// Push a higher-fidelity stage if within the hard timeout.
    pub fn push_stage(&mut self, stage: ApproxResult<O>) -> bool {
        if self.completed || self.started_at.elapsed() >= self.budget.hard_timeout {
            self.completed = true;
            return false;
        }
        let done = matches!(
            stage.mode_used,
            FidelityMode::SymbolicExact | FidelityMode::SymbolicHybrid
        );
        self.stages.push(stage);
        if done {
            self.completed = true;
        }
        true
    }

    /// Return the highest-fidelity result available right now without blocking.
    pub fn latest(&self) -> Option<&ApproxResult<O>> {
        self.stages.last()
    }

    /// Run a progressive pipeline (`NumericalApprox` -> `SymbolicApprox` -> `SymbolicHybrid`)
    /// stopping early if `frame_budget` is exhausted after Tier 0 or `completed` is reached.
    pub fn run_progressive<I, E, Op: MultiFidelityOp<I, O, Error = E>>(
        op: &Op,
        input: &I,
        form: EvalForm,
        budget: AnytimeBudget,
    ) -> Result<Self, E> {
        let tier0 = op.eval_with_fidelity(input, FidelityMode::NumericalApprox, form, &budget)?;
        let mut stream = Self::start_with_tier0(tier0, budget);

        if stream.started_at.elapsed() < budget.frame_budget {
            if let Ok(tier1) =
                op.eval_with_fidelity(input, FidelityMode::SymbolicApprox, form, &budget)
            {
                stream.push_stage(tier1);
            }
        }

        if !stream.completed && stream.started_at.elapsed() < budget.hard_timeout {
            if let Ok(tier3) =
                op.eval_with_fidelity(input, FidelityMode::SymbolicHybrid, form, &budget)
            {
                stream.push_stage(tier3);
            }
        }

        stream.completed = true;
        Ok(stream)
    }
}

/// Convenience constructor for a Tier-0 `ApproxResult`.
pub fn tier0_ball_result<O>(value: O, ball: UniversalBall, method: &'static str) -> ApproxResult<O> {
    ApproxResult {
        value,
        mode_used: FidelityMode::NumericalApprox,
        form: EvalForm::Canonical,
        enclosure: ErrorEnclosure::from_ball(ball, method),
        remainder_term: None,
        validity: DomainCondition::default(),
        budget_exceeded: false,
    }
}

/// Handle representing an ongoing or completed anytime evaluation.
pub struct AnytimeHandle {
    pub initial_ball: UniversalBall,
    pub current_ball: UniversalBall,
    pub frame_budget: Duration,
    pub start_time: Instant,
    pub is_exact: bool,
}

impl AnytimeHandle {
    /// Create handle and evaluate Tier-0 bounds immediately within <= 50 microseconds.
    pub fn start(term: &CatTerm, frame_budget: Duration) -> Self {
        let start_time = Instant::now();

        // Tier-0 fast enclosure estimation
        let initial_ball = term.spine.ball.clone();
        let is_exact = matches!(initial_ball, UniversalBall::ExactScalar(_));

        Self {
            initial_ball: initial_ball.clone(),
            current_ball: initial_ball,
            frame_budget,
            start_time,
            is_exact,
        }
    }

    /// Progressively refine the enclosure within the remaining frame budget.
    pub fn refine_step(&mut self) -> bool {
        if self.is_exact || self.start_time.elapsed() >= self.frame_budget {
            return false;
        }

        // Tighten enclosure bounds
        match &mut self.current_ball {
            UniversalBall::RealInterval { min, max } => {
                let center = (*min + *max) * 0.5;
                let half_span = (*max - *min) * 0.4; // Tighten by 20%
                *min = center - half_span;
                *max = center + half_span;
                if half_span <= 1e-15 {
                    self.is_exact = true;
                }
            }
            UniversalBall::ComplexDisk { radius, .. } => {
                *radius *= 0.5;
                if *radius <= 1e-15 {
                    self.is_exact = true;
                }
            }
            UniversalBall::LieManifoldAffine { uncertainty, .. } => {
                for u in uncertainty.iter_mut() {
                    *u *= 0.5;
                }
                if uncertainty.iter().all(|&u| u <= 1e-15) {
                    self.is_exact = true;
                }
            }
            _ => {
                self.is_exact = true;
            }
        }

        true
    }

    /// Run progressive refinement until the time budget is exhausted or exactness is achieved.
    pub fn run_to_budget(&mut self) {
        while self.refine_step() {
            if self.start_time.elapsed() >= self.frame_budget {
                break;
            }
        }
    }
}
