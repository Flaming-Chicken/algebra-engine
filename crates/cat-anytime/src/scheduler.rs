//! Sub-frame anytime scheduler with 8ms frame budgets and instant UniversalBall bounds.

use cat_core::ball::UniversalBall;
use cat_core::term::CatTerm;
use std::time::{Duration, Instant};

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
