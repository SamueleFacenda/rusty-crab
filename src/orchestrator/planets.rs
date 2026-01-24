use std::collections::HashSet;
use common_game::components::planet::Planet;
use common_game::components::resource::BasicResourceType::Carbon;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::{unbounded, Receiver, Sender};
use crate::orchestrator::example_explorer::Explorer;
use crate::orchestrator::orchestrator::{Orchestrator, PlanetHandle};
use HWHAB;
use air_fryer;
// use carbonium;
use one_million_crabs;

#[allow(dead_code)]
impl<T: Explorer> Orchestrator<T>{
    pub fn add_planet(
        &mut self,
        planet: Planet,
        tx: Sender<OrchestratorToPlanet>,
        rx: Receiver<PlanetToOrchestrator>,
        tx_explorer: Sender<ExplorerToPlanet>,
    ) -> Result<(), String> {
        let id = planet.id();
        if self.planets.contains_key(&id) {
            return Err(format!("Planet {} already exists", id));
        }

        self.planets.insert(id, PlanetHandle {
            planet: Some(planet),
            neighbors: HashSet::new(),
            thread_handle: None,
            tx,
            rx,
            tx_explorer,
        });
        Ok(())
    }

    // Function to connect planets.
    // All planets should already be in the list from initialization
    pub(crate) fn connect_planets (&mut self, planet1_id: ID, planet2_id: ID) -> Result<(), String> {
        if planet1_id == planet2_id {
            return Ok(())
        }

        if !self.planets.contains_key(&planet1_id) {
            return Err(format!("Planet {} does not exist", planet1_id));
        }
        if !self.planets.contains_key(&planet2_id) {
            return Err(format!("Planet {} does not exist", planet2_id));
        }

        // Unwrap is safe after check
        self.planets.get_mut(&planet1_id).unwrap().neighbors.insert(planet2_id);
        self.planets.get_mut(&planet2_id).unwrap().neighbors.insert(planet1_id);

        Ok(())
    }

    pub(crate) fn remove_planet_connections(&mut self, planet_id: ID) -> Result<(), String>{
        if !self.planets.contains_key(&planet_id) {
            return Err(format!("Planet {} does not exist", planet_id));
        }

        for (id, planet) in self.planets.iter_mut() {
            if *id == planet_id {
                planet.neighbors.clear();
            } else {
                planet.neighbors.remove(&planet_id);
            }
        }
        Ok(())
    }

    // Still uses 2.0.0
    // Must stay commented until updated, otherwise creates troubles:
    // package `common-game` is specified twice in the lockfile

    // fn create_planet_1(
    //     &mut self,
    //     id: ID
    // ) -> Result<(), String> {
    //     let (tx_orchestrator, rx_orchestrator) = unbounded();
    //     let (tx_planet, rx_planet) = unbounded();
    //     let (tx_explorer, rx_explorer) = unbounded();
    //
    //     let p = the_compiler_strikes_back::planet::create_planet(
    //         rx_orchestrator,
    //         tx_planet,
    //         rx_explorer,
    //         id
    //     );
    //     self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    // }

    // panic: out of oxygen
    fn create_planet_2(
        &mut self,
        id: ID
    ) -> Result<(), String> {
        let (tx_orchestrator, rx_orchestrator) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_explorer, rx_explorer) = unbounded();
        let p: Planet = match air_fryer::create_planet(
            id,
            air_fryer::PlanetAI::new(),
            (rx_orchestrator, tx_planet), // To be checked
            rx_explorer,

        ){
            Ok(p) => p,
            Err(e) => return Err(e)
        };
        self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    }

    fn create_planet_3(
        &mut self,
        id: ID
    ) -> Result<(), String> {
        let (tx_orchestrator, rx_orchestrator) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_explorer, rx_explorer) = unbounded();

        let p = rustrelli::create_planet(
            id,
            rx_orchestrator,
            tx_planet,
            rx_explorer,
            rustrelli::ExplorerRequestLimit::None, // Can be changed to FairShare
        );

        self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    }

    // Logically works but requires ssh which I'm too lazy to set up and will just wait for them to adapt using the crate

    // fn create_planet_4(
    //     &mut self,
    //     id: ID
    // ) -> Result<(), String> {
    //     let (tx_orchestrator, rx_orchestrator) = unbounded();
    //     let (tx_planet, rx_planet) = unbounded();
    //     let (tx_explorer, rx_explorer) = unbounded();
    //
    //     let p = carbonium::create_planet(
    //         id,
    //         rx_orchestrator,
    //         tx_planet,
    //         rx_explorer,
    //         rustrelli::ExplorerRequestLimit::None, // Can be changed to FairShare
    //     );
    //
    //     self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    // }

    fn create_planet_5(
        &mut self,
        id: ID
    ) -> Result<(), String> {
        let (tx_orchestrator, rx_orchestrator) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_explorer, rx_explorer) = unbounded();

        let p = match one_million_crabs::planet::create_planet(
            rx_orchestrator,
            tx_planet,
            rx_explorer,
            id,
        ){
            Ok(p) => {p}
            Err(e) => return Err(e)
        };

        self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    }

    // fulmini e saette
    fn create_planet_6(
        &mut self,
        id: ID
    ) -> Result<(), String> {
        let (tx_orchestrator, rx_orchestrator) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_explorer, rx_explorer) = unbounded();

        let p = match HWHAB::houston_we_have_a_borrow(
            rx_orchestrator,
            tx_planet,
            rx_explorer,
            id,
            HWHAB::RocketStrategy::Default, // (Disabled, Safe, EmergencyReserve, Default)
            Some(Carbon) // Any Option<BasicResourceType>, what a novel idea
        ){
            Ok(p) => {p}
            Err(e) => return Err(e)
        };

        self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    }

    fn create_planet_7(
        &mut self,
        id: ID
    ) -> Result<(), String> {
        let (tx_orchestrator, rx_orchestrator) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_explorer, rx_explorer) = unbounded();

        let p = rust_eze::create_planet(
            id,
            rx_orchestrator,
            tx_planet,
            rx_explorer,
        );

        self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    }

    // Call all 7 planet creator functions, stacks errors if any
    pub (crate) fn add_planets(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();

        for id in 1..=7 {
            let result = match id {
                // 1 => self.create_planet_1(id),
                2 => self.create_planet_2(id),
                3 => self.create_planet_3(id),
                // 4 => self.create_planet_4(id),
                5 => self.create_planet_5(id),
                6 => self.create_planet_6(id),
                7 => self.create_planet_7(id),
                _ => {Ok(())}, // Placeholder for planets 1, 4
            };

            if let Err(e) = result {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" | "))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::orchestrator::example_explorer::ExampleExplorer;
    use crate::orchestrator::orchestrator::Orchestrator;

    // Waiting for update

    // #[test]
    // fn verify_planet_1(){
    //     let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
    //     let result = orchestrator.create_planet_1(1);
    //     assert!(orchestrator.planets.contains_key(&1));
    //     assert_eq!(result, Ok(()));
    // }

    #[test]
    fn verify_planet_2(){
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let result = orchestrator.create_planet_2(2);
        assert!(orchestrator.planets.contains_key(&2));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_planet_3(){
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let result = orchestrator.create_planet_3(3);
        assert!(orchestrator.planets.contains_key(&3));
        assert_eq!(result, Ok(()));
    }

    // Waiting for update

    // #[test]
    // fn verify_planet_4(){
    //     let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
    //     let result = orchestrator.create_planet_4(4);
    //     assert!(orchestrator.planets.contains_key(&4));
    //     assert_eq!(result, Ok(()));
    // }

    #[test]
    fn verify_planet_5(){
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let result = orchestrator.create_planet_5(5);
        assert!(orchestrator.planets.contains_key(&5));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_planet_6(){
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let result = orchestrator.create_planet_6(6);
        assert!(orchestrator.planets.contains_key(&6));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_planet_7(){
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let result = orchestrator.create_planet_7(7);
        assert!(orchestrator.planets.contains_key(&7));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_add_all_planets() {
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let result = orchestrator.add_planets();
        assert_eq!(result, Ok(()));

        for id in 1..=7 {
            if id != 1 && id != 4 { // Waiting for 1 and 4
                assert!(
                    orchestrator.planets.contains_key(&id),
                    "Planet with ID {} missing from the orchestrator", id
                );
            }
        }
    }
}