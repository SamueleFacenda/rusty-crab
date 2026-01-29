use common_game::utils::ID;
use std::collections::{HashMap, HashSet};

/// Galaxy topology container, only manages the connections between planets.
pub(crate) struct Galaxy {
    connections: HashMap<ID, HashSet<ID>>,
}

impl Galaxy {
    pub fn make_fully_connected(ids: &[ID]) -> Result<Self, String> {
        let mut connections: HashMap<ID, HashSet<ID>> = HashMap::new();

        for &id in ids {
            let mut connected_planets = HashSet::new();
            // Connect to all previously added planets
            for (prev, prev_connection) in &mut connections {
                prev_connection.insert(id);
                connected_planets.insert(*prev);
            }

            if connections.contains_key(&id) {
                return Err(format!("Duplicate planet ID found: {id}"));
            }
            connections.insert(id, connected_planets);
        }
        Ok(Galaxy { connections })
    }

    pub fn make_circular(ids: &[ID]) -> Result<Self, String> {
        let mut connections: HashMap<ID, HashSet<ID>> =
            ids.iter().map(|&id| (id, HashSet::new())).collect();
        if connections.len() != ids.len() {
            return Err("Duplicate planet IDs found".to_string());
        }

        // connect to the next planet
        for i in 0..ids.len() {
            let current_id = ids[i];
            let next_id = ids[(i + 1) % ids.len()];

            // Unwrap is safe here because we initialized the connections map with all ids
            connections.get_mut(&current_id).unwrap().insert(next_id);
            connections.get_mut(&next_id).unwrap().insert(current_id);
        }

        Ok(Galaxy { connections })
    }

    pub fn get_planets(&self) -> Vec<ID> {
        self.connections.keys().copied().collect()
    }

    pub fn are_planets_connected(&self, a: ID, b: ID) -> bool {
        if let Some(neighbors) = self.connections.get(&a) {
            neighbors.contains(&b)
        } else {
            false
        }
    }

    pub fn remove_planet(&mut self, id: ID) {
        self.connections.remove(&id);
        for neighbors in self.connections.values_mut() {
            neighbors.remove(&id);
        }
    }
}

#[allow(clippy::wildcard_imports)] // It's just tests
mod test {
    use super::*;

    fn get_dummy_ids() -> Vec<ID> {
        vec![1, 2, 3, 4, 5]
    }

    #[test]
    fn test_fully_connected() {
        let galaxy = Galaxy::make_fully_connected(&get_dummy_ids());
        assert!(galaxy.is_ok());
    }

    #[test]
    fn test_fully_connected_connections() {
        let galaxy = Galaxy::make_fully_connected(&get_dummy_ids()).unwrap();

        for planet in galaxy.get_planets() {
            for other_planet in galaxy.get_planets() {
                if planet != other_planet {
                    assert!(galaxy.are_planets_connected(planet, other_planet));
                }
            }
        }
    }

    #[test]
    fn test_circular() {
        let galaxy = Galaxy::make_circular(&get_dummy_ids());
        assert!(galaxy.is_ok());
    }

    #[test]
    fn test_circular_connections() {
        let galaxy = Galaxy::make_circular(&get_dummy_ids()).unwrap();
        let mut planets = galaxy.get_planets();
        planets.sort();
        let num_planets = planets.len();

        for i in 0..num_planets {
            let planet = planets[i];
            let next_planet = planets[(i + 1) % num_planets];
            let prev_planet = planets[(i + num_planets - 1) % num_planets];

            assert!(galaxy.are_planets_connected(planet, next_planet));
            assert!(galaxy.are_planets_connected(planet, prev_planet));
        }
    }

    #[test]
    fn test_remove_planet() {
        let mut galaxy = Galaxy::make_fully_connected(&get_dummy_ids()).unwrap();
        let planet_to_remove = 3;

        galaxy.remove_planet(planet_to_remove);

        assert!(!galaxy.get_planets().contains(&planet_to_remove));

        for planet in galaxy.get_planets() {
            assert!(!galaxy.are_planets_connected(planet, planet_to_remove));
        }
    }
}
