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
pub fn tier0_ball_result<O>(
    value: O,
    ball: UniversalBall,
    method: &'static str,
) -> ApproxResult<O> {
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

    /// Adopt `candidate` as the current enclosure if it is sound: contained in the current one and
    /// strictly smaller. Returns whether it was adopted.
    ///
    /// An enclosure may only shrink because a more precise evaluation justified it, so the caller
    /// supplies the re-evaluated ball. A candidate that is not contained in the current ball, or
    /// not tighter, is rejected and the current ball is kept. Only real intervals, complex disks
    /// and Lie-manifold balls can be compared; other kinds are never replaced.
    pub fn refine_with(&mut self, candidate: UniversalBall) -> bool {
        const SLACK: f64 = 1e-12;
        let accepted = match (&self.current_ball, &candidate) {
            (
                UniversalBall::RealInterval { min, max },
                UniversalBall::RealInterval {
                    min: new_min,
                    max: new_max,
                },
            ) => {
                *new_min >= *min - SLACK
                    && *new_max <= *max + SLACK
                    && (new_max - new_min) < (max - min)
            }
            (
                UniversalBall::ComplexDisk {
                    center_re,
                    center_im,
                    radius,
                },
                UniversalBall::ComplexDisk {
                    center_re: new_re,
                    center_im: new_im,
                    radius: new_radius,
                },
            ) => {
                let offset = (new_re - center_re).hypot(new_im - center_im);
                offset + new_radius <= *radius + SLACK && new_radius < radius
            }
            (
                UniversalBall::LieManifoldAffine {
                    center,
                    uncertainty,
                    ..
                },
                UniversalBall::LieManifoldAffine {
                    center: new_center,
                    uncertainty: new_uncertainty,
                    ..
                },
            ) => {
                let contained = (0..6).all(|i| {
                    (new_center[i] - center[i]).abs() + new_uncertainty[i] <= uncertainty[i] + SLACK
                });
                let tighter = (0..6).any(|i| new_uncertainty[i] < uncertainty[i]);
                contained && tighter
            }
            _ => false,
        };
        if accepted {
            self.current_ball = candidate;
            self.is_exact = match &self.current_ball {
                UniversalBall::RealInterval { min, max } => max - min <= 1e-15,
                UniversalBall::ComplexDisk { radius, .. } => *radius <= 1e-15,
                UniversalBall::LieManifoldAffine { uncertainty, .. } => {
                    uncertainty.iter().all(|&u| u <= 1e-15)
                }
                _ => self.is_exact,
            };
        }
        accepted
    }

    /// One refinement step: ask `reevaluate` for a more precise ball and adopt it if it is sound.
    /// Returns `false` when the handle is exact, the frame budget is spent, or the evaluator
    /// produced nothing usable.
    pub fn refine_step<F>(&mut self, reevaluate: &mut F) -> bool
    where
        F: FnMut(&UniversalBall) -> Option<UniversalBall>,
    {
        if self.is_exact || self.start_time.elapsed() >= self.frame_budget {
            return false;
        }
        match reevaluate(&self.current_ball) {
            Some(candidate) => self.refine_with(candidate),
            None => false,
        }
    }

    /// Run refinement with `reevaluate` until the time budget is exhausted, the ball is exact, or
    /// the evaluator stops improving the enclosure.
    pub fn run_to_budget<F>(&mut self, mut reevaluate: F)
    where
        F: FnMut(&UniversalBall) -> Option<UniversalBall>,
    {
        while self.refine_step(&mut reevaluate) {
            if self.start_time.elapsed() >= self.frame_budget {
                break;
            }
        }
    }
}
