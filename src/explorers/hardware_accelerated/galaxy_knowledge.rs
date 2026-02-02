use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;

#[derive(Debug)]
struct PlanetKnowledge {
    basic_resources: HashSet<BasicResourceType>,
    complex_resources: HashSet<ComplexResourceType>,
    n_charged_cells: u32
}

impl PlanetKnowledge {
    pub fn new() -> Self {
        PlanetKnowledge { basic_resources: HashSet::new(), complex_resources: HashSet::new(), n_charged_cells: 0 }
    }
}

/// Topology and state of the planets
#[derive(Debug)]
pub(super) struct GalaxyKnowledge {
    connections: HashMap<ID, HashSet<ID>>,
    planets_knowledge: HashMap<ID, PlanetKnowledge>,
    max_charged_cells_per_planet: u32
}

impl GalaxyKnowledge {
    pub fn new() -> Self {
        GalaxyKnowledge {
            connections: HashMap::new(),
            planets_knowledge: HashMap::new(),
            max_charged_cells_per_planet: 0
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

    pub fn set_n_charged_cells(&mut self, id: ID, n: u32) {
        if let Some(planet_knowledge) = self.planets_knowledge.get_mut(&id) {
            planet_knowledge.n_charged_cells = n;
            self.max_charged_cells_per_planet = self.max_charged_cells_per_planet.max(n);
        }
    }

    pub fn has_planet(&self, id: ID) -> bool { self.connections.contains_key(&id) }

    /// This is a deterministic lower bound estimate of sunrays received
    pub fn estimate_sunrays_received(&self, new_state: &GalaxyKnowledge) -> u32 {
        self.count_comparison_predicate(new_state, |old, new| new.n_charged_cells > old.n_charged_cells)
    }

    /// This is a deterministic lower bound estimate of asteroids received (assumes no cells were used for
    /// building) Maybe it's not reliable in a multi-explorer scenario
    pub fn estimate_asteroids_received(&self, new_state: &GalaxyKnowledge) -> u32 {
        self.count_comparison_predicate(new_state, |old, new| new.n_charged_cells < old.n_charged_cells)
    }

    fn count_comparison_predicate(
        &self,
        b: &GalaxyKnowledge,
        pred: impl Fn(&PlanetKnowledge, &PlanetKnowledge) -> bool
    ) -> u32 {
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

    /// From 0 to 1, based on already available charged cells.
    /// Planets that would resist longer without help are considered more reliable.
    pub fn get_planet_reliability(&self, id: ID) -> f32 {
        let planet_knowledge = match self.planets_knowledge.get(&id) {
            Some(pk) => pk,
            None => return 0.0
        };

        let mut out = 0.0;

        if self.can_have_rocket(id) {
            out += 0.5;
        }

        if self.has_planet_cell_unbounded(id) {
            out += 0.2
        }

        if self.max_charged_cells_per_planet != 0 {
            out += (planet_knowledge.n_charged_cells as f32 / self.max_charged_cells_per_planet as f32) * 0.3;
        }

        return out;
    }

    pub fn get_n_planets(&self) -> usize { self.connections.len() }

    pub fn get_planet_neighbours(&self, planet_id: ID) -> Option<&HashSet<ID>> {
        if let Some(neighbors) = self.connections.get(&planet_id) { Some(neighbors) } else { None }
    }

    pub fn get_planet_ids(&self) -> Vec<ID> { self.connections.keys().cloned().collect() }

    // Infer the planet type from recipes
    fn can_have_rocket(&self, id: ID) -> bool {
        if let Some(planet_knowledge) = self.planets_knowledge.get(&id) {
            planet_knowledge.basic_resources.len() == 1
        } else {
            false
        }
    }

    // Infer the planet type from recipes
    fn has_planet_cell_unbounded(&self, id: ID) -> bool {
        if let Some(planet_knowledge) = self.planets_knowledge.get(&id) {
            planet_knowledge.complex_resources.len() == 0
        } else {
            false
        }
    }

    pub fn produces_basic_resource(&self, id: ID, res: BasicResourceType) -> bool {
        if let Some(planet_knowledge) = self.planets_knowledge.get(&id) {
            planet_knowledge.basic_resources.contains(&res)
        } else {
            false
        }
    }

    pub fn supports_combination_rule(&self, id: ID, rule: ComplexResourceType) -> bool {
        if let Some(planet_knowledge) = self.planets_knowledge.get(&id) {
            planet_knowledge.complex_resources.contains(&rule)
        } else {
            false
        }
    }

    pub fn get_n_charged_cells(&self, id: ID) -> u32 {
        self.planets_knowledge.get(&id).map_or(0, |pk| pk.n_charged_cells)
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
        gk.set_n_charged_cells(1, 2);
        gk.set_n_charged_cells(2, 1);
        gk
    }

    #[test]
    fn test_planet_reliability() {
        let gk = get_dummy_knowledge();
        let r1 = gk.get_planet_reliability(1);
        let r2 = gk.get_planet_reliability(2);
        assert!(r1 > r2);
    }

    #[test]
    fn test_estimate_sunrays_received() {
        let old_gk = get_dummy_knowledge();
        let mut new_gk = get_dummy_knowledge();
        new_gk.set_n_charged_cells(2, 2); // planet has 2 charged cells
        let sunrays = old_gk.estimate_sunrays_received(&new_gk);
        assert_eq!(sunrays, 1);
    }

    #[test]
    fn test_estimate_asteroids_received() {
        let old_gk = get_dummy_knowledge();
        let mut new_gk = get_dummy_knowledge();
        new_gk.set_n_charged_cells(1, 1); // planet 1 lost a charged cell
        let asteroids = old_gk.estimate_asteroids_received(&new_gk);
        assert_eq!(asteroids, 1);
    }
}
