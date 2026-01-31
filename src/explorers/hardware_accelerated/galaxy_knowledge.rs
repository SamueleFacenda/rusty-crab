use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;

struct PlanetKnowledge {
    basic_resources: HashSet<BasicResourceType>,
    complex_resources: HashSet<ComplexResourceType>,
    has_rocket: bool,
    n_charged_cells: u32,
    has_charged_cells_limit: bool, // limited to one
}

impl PlanetKnowledge {
    pub fn new() -> Self {
        PlanetKnowledge {
            basic_resources: HashSet::new(),
            complex_resources: HashSet::new(),
            has_rocket: false,
            n_charged_cells: 0,
            has_charged_cells_limit: false,
        }
    }
}

/// Topology and state of the planets
pub(super) struct GalaxyKnowledge {
    connections: HashMap<ID, HashSet<ID>>,
    planets_knowledge: HashMap<ID, PlanetKnowledge>,
    max_charged_cells_per_planet: u32,
}

impl GalaxyKnowledge {
    pub fn new() -> Self {
        GalaxyKnowledge {
            connections: HashMap::new(),
            planets_knowledge: HashMap::new(),
            max_charged_cells_per_planet: 0,
        }
    }


    pub fn add_planet(&mut self, id: ID) {
        self.connections.insert(id, HashSet::new());
        self.planets_knowledge.insert(id, PlanetKnowledge::new());
    }

    pub fn add_planets_connection(&mut self, a: ID, b: ID) {
        if let Some(neighbors) = self.connections.get_mut(&a) {
            neighbors.insert(b);
        }
        if let Some(neighbors) = self.connections.get_mut(&b) {
            neighbors.insert(a);
        }
    }

    pub fn set_planet_basic_resources(&mut self, id: ID, res: HashSet<BasicResourceType>) {
        if let Some(planet_knowledge) = self.planets_knowledge.get_mut(&id) {
            planet_knowledge.basic_resources = res;
        }
    }

    pub fn set_planet_combination_rules(&mut self, id: ID, rules: HashSet<ComplexResourceType>) {
        if let Some(planet_knowledge) = self.planets_knowledge.get_mut(&id) {
            planet_knowledge.complex_resources = rules;
        }
    }

    pub fn set_has_rocket(&mut self, id: ID, rocket: bool) {
        if let Some(planet_knowledge) = self.planets_knowledge.get_mut(&id) {
            planet_knowledge.has_rocket = rocket;
        }
    }

    pub fn set_n_charged_cells(&mut self, id: ID, n: u32) {
        if let Some(planet_knowledge) = self.planets_knowledge.get_mut(&id) {
            planet_knowledge.n_charged_cells = n;
            self.max_charged_cells_per_planet = self.max_charged_cells_per_planet.max(n);
        }
    }

    pub fn set_has_charged_cells_limit(&mut self, id: ID, limited: bool) {
        if let Some(planet_knowledge) = self.planets_knowledge.get_mut(&id) {
            planet_knowledge.has_charged_cells_limit = limited;
        }
    }

    pub fn has_planet(&self, id: ID) -> bool {
        self.connections.contains_key(&id)
    }

    /// This is a deterministic lower bound estimate of sunrays received
    pub fn estimate_sunrays_received(&self, new_state: &GalaxyKnowledge) -> u32 {
        self.count_comparison_predicate(new_state, |old, new| {
            new.has_rocket && !old.has_rocket ||
            new.n_charged_cells > old.n_charged_cells
        })
    }

    /// This is a deterministic lower bound estimate of asteroids received (assumes no cells were used for building)
    /// Maybe it's not reliable in a multi-explorer scenario
    pub fn estimate_asteroids_received(&self, new_state: &GalaxyKnowledge) -> u32 {
        self.count_comparison_predicate(new_state, |old, new| {
            (!new.has_rocket && old.has_rocket) ||
            (new.n_charged_cells < old.n_charged_cells)
        })
    }

    fn count_comparison_predicate(&self, b: &GalaxyKnowledge, pred: impl Fn(&PlanetKnowledge, &PlanetKnowledge) -> bool) -> u32 {
        let mut count = 0;
        for (id, planet_knowledge) in &self.planets_knowledge {
            if let Some(other_knowledge) = b.planets_knowledge.get(id) {
                if pred(planet_knowledge, other_knowledge) {
                    count += 1;
                }
            }
        }
        count
    }

    /// From 0 to 1, based on already available rockets and charged cells.
    /// Planets that would resist longer without help are considered more reliable.
    pub fn get_planet_reliability(&self, id: ID) -> f32 {
        let planet_knowledge = match self.planets_knowledge.get(&id) {
            Some(pk) => pk,
            None => return 0.0,
        };
        let mut out = 0.0;
        if planet_knowledge.has_rocket {
            out += 0.5;
        }
        if planet_knowledge.n_charged_cells > 0 {
            out += 0.1;
        }
        if !planet_knowledge.has_charged_cells_limit {
            out += 0.1; // can get more charged cells
            out += (planet_knowledge.n_charged_cells as f32) / (self.max_charged_cells_per_planet as f32) * 0.3;
        }
        return out;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn get_dummy_knowledge() -> GalaxyKnowledge {
        let mut gk = GalaxyKnowledge::new();
        gk.add_planet(1);
        gk.add_planet(2);
        gk.add_planets_connection(1, 2);
        gk.set_has_rocket(1, false);
        gk.set_n_charged_cells(1, 2);
        gk.set_has_charged_cells_limit(1, false);
        gk.set_has_rocket(2, true);
        gk.set_n_charged_cells(2, 1);
        gk.set_has_charged_cells_limit(2, true);
        gk
    }

    #[test]
    fn test_planet_reliability() {
        let gk = get_dummy_knowledge();
        let r1 = gk.get_planet_reliability(1);
        let r2 = gk.get_planet_reliability(2);
        assert!(r2 > r1);
    }

    #[test]
    fn test_estimate_sunrays_received() {
        let mut old_gk = get_dummy_knowledge();
        let mut new_gk = get_dummy_knowledge();
        new_gk.set_has_rocket(1, true); // planet 1 got a rocket
        new_gk.set_n_charged_cells(2, 2); // planet 2 got more charged cells
        let sunrays = old_gk.estimate_sunrays_received(&new_gk);
        assert_eq!(sunrays, 2);
    }

    #[test]
    fn test_estimate_asteroids_received() {
        let mut old_gk = get_dummy_knowledge();
        let mut new_gk = get_dummy_knowledge();
        new_gk.set_has_rocket(2, false); // planet 2 lost its rocket
        new_gk.set_n_charged_cells(1, 1); // planet 1 lost a charged cell
        let asteroids = old_gk.estimate_asteroids_received(&new_gk);
        assert_eq!(asteroids, 2);
    }
}