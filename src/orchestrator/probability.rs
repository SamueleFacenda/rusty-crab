use crate::app::AppConfig;

pub(crate) struct ProbabilityCalculator;

impl ProbabilityCalculator {
    pub(crate) fn get_asteroid_probability(time: u32) -> f32 {
        // A sigmoid function that starts with y=initial_asteroid_probability
        let p_start = AppConfig::get().initial_asteroid_probability;
        let probability = AppConfig::get().asteroid_probability;
        let t0 = (1.0 / probability) * ((1.0 - p_start) / p_start).ln();
        1.0 / (1.0 + (-probability * (time as f32 - t0)).exp())
    }

    pub(crate) fn get_sunray_probability(_time: u32) -> f32 {
        AppConfig::get().sunray_probability
    }
}

#[cfg(test)]
mod tests {
    use crate::orchestrator::probability::ProbabilityCalculator;

    #[test]
    fn verify_probabilities() {
        // verify the initial value and that the probability tends to 1
        let asteroid_0 = ProbabilityCalculator::get_asteroid_probability(0);
        let sunray_0 = ProbabilityCalculator::get_sunray_probability(0);
        // println!("0: {}, time: {}", asteroid_0, orchestrator.time);
        assert!(asteroid_0 < 0.01001);
        assert!(asteroid_0 > 0.0099);
        assert_eq!(sunray_0, 0.1);
        let asteroid_100 = ProbabilityCalculator::get_asteroid_probability(100);
        let sunray_100 = ProbabilityCalculator::get_sunray_probability(100);
        // println!("100: {}, time: {}", asteroid_100, orchestrator.time);
        assert!(asteroid_100 <= 0.03);
        assert!(asteroid_100 >= 0.02);
        assert_eq!(sunray_100, 0.1);
        let asteroid_1000 = ProbabilityCalculator::get_asteroid_probability(1000);
        let sunray_1000 = ProbabilityCalculator::get_sunray_probability(1000);
        // println!("1000: {}, time: {}", asteroid_1000, orchestrator.time);
        assert!(asteroid_1000 >= 0.9);
        assert_eq!(sunray_1000, 0.1);
    }
}
