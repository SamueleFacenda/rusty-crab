use crate::app::AppConfig;

pub(super) struct ProbabilityEstimator {
    estimate: f32
}

impl ProbabilityEstimator {
    pub fn new() -> Self { ProbabilityEstimator { estimate: -1.0 } }

    #[allow(clippy::cast_precision_loss)] // f32 is precise enough for our needs
    pub fn update(&mut self, n_planets: u32, n_affected: u32) {
        let new_prob = n_affected as f32 / n_planets as f32;
        let sensibility = if self.estimate < 0f32 { 1.0 } else { Self::get_sensibility() };

        self.estimate = self.estimate * (1.0 - sensibility) + new_prob * sensibility;
    }
    pub fn get_probability(&self) -> f32 { self.estimate }

    fn get_sensibility() -> f32 { AppConfig::get().explorer_probability_estimator_sensitivity }
}

#[cfg(test)]
mod test {
    use super::ProbabilityEstimator;

    #[test]
    fn test_few_data() {
        let mut estimator = ProbabilityEstimator::new();

        estimator.update(100, 50);
        assert!(estimator.get_probability() < 0.51);
        assert!(estimator.get_probability() > 0.49);

        estimator.update(100, 0);
        assert!(estimator.get_probability() < 0.50);
    }

    #[test]
    fn test_more_data() {
        let mut estimator = ProbabilityEstimator::new();

        for _ in 0..100 {
            estimator.update(100, 0);
        }
        assert!(estimator.get_probability() < 0.01);
        assert!(estimator.get_probability() > -0.01);

        for _ in 0..100 {
            estimator.update(100, 100);
        }
        assert!(estimator.get_probability() < 1.0);
        assert!(estimator.get_probability() > 0.5);
    }
}
