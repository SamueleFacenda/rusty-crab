use common_game::components::planet::Planet;
use common_game::components::resource::BasicResourceType::Carbon;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use HWHAB;
use air_fryer;
// use carbonium;
use one_million_crabs;

#[derive(Copy, Clone)]
pub(crate) enum PlanetType {
    PanicOutOfOxygen,
    TheCompilerStrikesBack,
    Rustrelli,
    Carbonium,
    OneMillionCrabs,
    HoustonWeHaveABorrow,
    RustEze,
}

pub(crate) struct PlanetFactory;

impl PlanetFactory {
    pub(crate) fn make_planet(
        planet_type: PlanetType,
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        match planet_type {
            PlanetType::PanicOutOfOxygen => {
                Self::create_panic_out_of_oxygen_planet(id, sender, receiver, explorer_receiver)
            }
            PlanetType::Rustrelli => {
                Self::create_rustrelli_planet(id, sender, receiver, explorer_receiver)
            }
            PlanetType::TheCompilerStrikesBack => {
                Self::create_the_compiler_strikes_back_planet(id, sender, receiver, explorer_receiver)
            }
            PlanetType::Carbonium => {
                Self::create_carbonium_planet(id, sender, receiver, explorer_receiver)
            }
            PlanetType::OneMillionCrabs => {
                Self::create_one_million_crabs_planet(id, sender, receiver, explorer_receiver)
            }
            PlanetType::HoustonWeHaveABorrow => {
                Self::create_houston_we_have_a_borrow_planet(id, sender, receiver, explorer_receiver)
            }
            PlanetType::RustEze => {
                Self::create_rust_eze_planet(id, sender, receiver, explorer_receiver)
            }
        }
    }

    // Still uses 2.0.0
    // Must stay commented until updated, otherwise creates troubles:
    // package `common-game` is specified twice in the lockfile
    fn create_the_compiler_strikes_back_planet(
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        todo!()
        // the_compiler_strikes_back::planet::create_planet(
        //     receiver,
        //     sender,
        //     explorer_receiver,
        //     id
        // )
    }

    fn create_panic_out_of_oxygen_planet(
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        air_fryer::create_planet(
            id,
            air_fryer::PlanetAI::new(),
            (receiver, sender), // To be checked
            explorer_receiver,
        )
    }

    fn create_rustrelli_planet(
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        Ok(rustrelli::create_planet(
            id,
            receiver,
            sender,
            explorer_receiver,
            rustrelli::ExplorerRequestLimit::None, // Can be changed to FairShare
        ))
    }


    // Logically works but requires ssh which I'm too lazy to set up and will just wait for them to adapt using the crate
    fn create_carbonium_planet(
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        todo!()
        // carbonium::create_planet(
        //     id,
        //     receiver,
        //     sender,
        //     explorer_receiver,
        //     rustrelli::ExplorerRequestLimit::None, // Can be changed to FairShare
        // )
    }

    fn create_one_million_crabs_planet(
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        one_million_crabs::planet::create_planet(
            receiver,
            sender,
            explorer_receiver,
            id,
        )
    }


    // fulmini e saette
    fn create_houston_we_have_a_borrow_planet(
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        HWHAB::houston_we_have_a_borrow(
            receiver,
            sender,
            explorer_receiver,
            id,
            HWHAB::RocketStrategy::Default, // (Disabled, Safe, EmergencyReserve, Default)
            Some(Carbon) // Any Option<BasicResourceType>, what a novel idea
        )
    }

    fn create_rust_eze_planet(
        id: ID,
        sender: Sender<PlanetToOrchestrator>,
        receiver: Receiver<OrchestratorToPlanet>,
        explorer_receiver: Receiver<ExplorerToPlanet>,
    ) -> Result<Planet, String> {
        Ok(rust_eze::create_planet(
            id,
            receiver,
            sender,
            explorer_receiver,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crossbeam_channel::unbounded;
    use super::*;

    fn get_channels() -> (
        Sender<PlanetToOrchestrator>,
        Receiver<OrchestratorToPlanet>,
        Receiver<ExplorerToPlanet>,
    ) {
        let (tx_orch, rx_orch) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_explorer, rx_explorer) = unbounded();
        (tx_planet, rx_orch, rx_explorer)
    }

    #[test]
    fn test_panic_out_of_oxygen_planet_creation() {
        let (tx_planet, rx_orch, rx_explorer) = get_channels();

        let planet = PlanetFactory::make_planet(
            PlanetType::PanicOutOfOxygen,
            1,
            tx_planet,
            rx_orch,
            rx_explorer,
        );

        assert!(planet.is_ok());
    }

    #[test]
    fn test_rustrelli_planet_creation() {
        let (tx_planet, rx_orch, rx_explorer) = get_channels();

        let planet = PlanetFactory::make_planet(
            PlanetType::Rustrelli,
            2,
            tx_planet,
            rx_orch,
            rx_explorer,
        );

        assert!(planet.is_ok());
    }

    #[test]
    fn test_one_million_crabs_planet_creation() {
        let (tx_planet, rx_orch, rx_explorer) = get_channels();

        let planet = PlanetFactory::make_planet(
            PlanetType::OneMillionCrabs,
            3,
            tx_planet,
            rx_orch,
            rx_explorer,
        );

        assert!(planet.is_ok());
    }

    #[test]
    fn test_houston_we_have_a_borrow_planet_creation() {
        let (tx_planet, rx_orch, rx_explorer) = get_channels();

        let planet = PlanetFactory::make_planet(
            PlanetType::HoustonWeHaveABorrow,
            4,
            tx_planet,
            rx_orch,
            rx_explorer,
        );

        assert!(planet.is_ok());
    }

    #[test]
    fn test_rust_eze_planet_creation() {
        let (tx_planet, rx_orch, rx_explorer) = get_channels();

        let planet = PlanetFactory::make_planet(
            PlanetType::RustEze,
            5,
            tx_planet,
            rx_orch,
            rx_explorer,
        );

        assert!(planet.is_ok());
    }

    #[test]
    fn test_carbonium_planet_creation() {
        let (tx_planet, rx_orch, rx_explorer) = get_channels();

        let planet = PlanetFactory::make_planet(
            PlanetType::Carbonium,
            6,
            tx_planet,
            rx_orch,
            rx_explorer,
        );

        // Since Carbonium creation is not implemented, we expect an error
        assert!(planet.is_err());
    }

    #[test]
    fn test_the_compiler_strikes_back_planet_creation() {
        let (tx_planet, rx_orch, rx_explorer) = get_channels();

        let planet = PlanetFactory::create_the_compiler_strikes_back_planet(
            7,
            tx_planet,
            rx_orch,
            rx_explorer,
        );

        // Since The Compiler Strikes Back creation is not implemented, we expect an error
        assert!(planet.is_err());
    }

}