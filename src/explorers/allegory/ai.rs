use crate::explorers::allegory::knowledge::StrategyState::*;
use crate::explorers::allegory::knowledge::StrategyState;
use common_game::components::resource::ResourceType;
use common_game::protocols::{
    orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer},
    planet_explorer::{ExplorerToPlanet, PlanetToExplorer},
};
use crossbeam_channel::{Receiver, Sender, select};
use std::collections::{HashMap, HashSet};
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator::BagContentResponse;
use crate::explorers::allegory::explorer::AllegoryExplorer;
use crate::explorers::allegory::logging::{emit_info};

impl AllegoryExplorer {
    /// Function to execute a loop. Begins with galaxy exploration, then performs its algorithm.
    pub fn run_loop(&mut self) -> Result<(), String> {
        emit_info(self.id, "Starting turn".to_string());
        self.explore();
        self.perform_next_step();
        self.conclude_turn()?;
        emit_info(self.id, "Ending turn".to_string());
        Ok(())
    }


    fn explore(&mut self) {
        let mut explored_this_round = HashSet::new();
        loop {
            // Check termination condition
            let unexplored = self.knowledge.get_unexplored_planets();
            if unexplored.is_empty() {
                return;
            }

            // Request info for current planet: neighbors, cells, resources and combination if needed
            self.query_planet();
            explored_this_round.insert(self.current_planet_id);

            // Wait for neighbors answer
            loop {
                select! {
                    recv(self.rx_orchestrator) -> msg => {
                        match msg {
                            Ok(m) => {
                                let is_neighbors_response = matches!(&m, OrchestratorToExplorer::NeighborsResponse { .. });
                                if let Err(e) = self.handle_orchestrator_message(m) {
                                    log::error!("Error handling orchestrator message: {}", e);
                                }
                                if is_neighbors_response {
                                    break;
                                }
                            },
                            Err(_) => return, // Should not happen
                        }
                    },
                     recv(self.rx_planet) -> msg => {
                        match msg {
                            Ok(m) => {
                                if let Err(e) = self.handle_planet_message(m) {
                                    log::error!("Error handling planet message: {}", e);
                                }
                            },
                            Err(_) => return,
                        }
                    }
                }
            }

            // Move when neighbors information is updated to next unexplored planet
            let unexplored = self.knowledge.get_unexplored_from_hash(&explored_this_round);
            match self.find_first_unexplored(&unexplored) {
                None => return, // done exploring
                Some(id) => {
                    self.knowledge.set_destination(Some(id));
                    let next_hop = self.knowledge.get_next_hop(self.current_planet_id);
                    self.send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
                        explorer_id: self.id,
                        current_planet_id: self.current_planet_id,
                        dst_planet_id: next_hop,
                    });

                    // Wait for arrival confirmation
                    loop {
                        select! {
                            recv(self.rx_orchestrator) -> msg => {
                                if let Ok(m) = msg {
                                    let is_move = matches!(&m, OrchestratorToExplorer::MoveToPlanet { .. });
                                    if let Err(e) = self.handle_orchestrator_message(m) {
                                         log::error!("Error handling orchestrator message: {}", e);
                                    }
                                    if is_move {
                                        break; // Arrived
                                    }
                                } else { return; }
                            },
                             recv(self.rx_planet) -> msg => {
                                 if let Ok(m) = msg {
                                     if let Err(e) = self.handle_planet_message(m) {
                                          log::error!("Error handling planet message: {}", e);
                                     }
                                 }
                             }
                        }
                    }
                }
            }
        }
    }

    /// Explorer AI: decides what to do next
    /// - Exploring: Visits all planets, storing the specific information in the planet knowledge
    /// - Collecting: calculate the necessary resources to craft all the complex resources, sum it to the simple resources,
    /// travel along the planets to collect what needed to fulfill the task
    /// - crafting: create stuff from the list
    /// Should be called if the explorer is in automatic mode only
    fn perform_next_step(&mut self){
        let state = self.knowledge.get_current_state();
        // Manually cloning state to avoid borrow checker issues with self.knowledge
        let state = match state {
            StrategyState::Exploring => StrategyState::Exploring,
            StrategyState::Collecting => StrategyState::Collecting,
            StrategyState::Crafting => StrategyState::Crafting,
            StrategyState::Finished => StrategyState::Finished,
            StrategyState::Failed => StrategyState::Failed,
        };

        let (or_msg, pl_msg) = self.next_step(state);

        if let Some(msg) = or_msg {
            self.send_to_orchestrator(msg);
        }
        if let Some(msg) = pl_msg {
            self.send_to_planet(msg);
        }
    }

    fn conclude_turn(&mut self) -> Result<(), String> {
        match self.tx_orchestrator.send(
            BagContentResponse {
                explorer_id: self.id,
                bag_content: BagContent::from_bag(
                    &self.bag
                )
            }
        ){
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error sending to orchestrator: {}", e))
        }
    }

    /// Deprecated version of next step for one communication per turn
    pub fn next_step(
        &mut self,
        state: StrategyState,
    ) -> (
        Option<ExplorerToOrchestrator<BagContent>>,
        Option<ExplorerToPlanet>,
    ) {
        let mut destination = self.knowledge.get_target_planet();
        // Duplicating match to save one turn in case destination is reached
        match destination {
            Some(id) => {
                if id == self.current_planet_id {
                    destination = None;
                    self.knowledge.set_destination(None);
                }
            }
            _ => {}
        }
        match destination {
            None => {
                // If not travelling, decide what to do next
                match state {
                    Exploring => {
                        // Gather information on current planet if missing; otherwise, move
                        if let Some(pk) =
                            self.knowledge.get_planet_knowledge(self.current_planet_id)
                        {
                            if pk.get_neighbors().is_empty() {
                                return (
                                    Some(ExplorerToOrchestrator::NeighborsRequest {
                                        explorer_id: self.id,
                                        current_planet_id: self.current_planet_id,
                                    }),
                                    None,
                                );
                            }
                            if pk.get_resource_type().is_empty() {
                                return (
                                    None,
                                    Some(ExplorerToPlanet::SupportedResourceRequest {
                                        explorer_id: self.id,
                                    }),
                                );
                            }
                            if pk.get_combinations().is_empty() {
                                return (
                                    None,
                                    Some(ExplorerToPlanet::SupportedCombinationRequest {
                                        explorer_id: self.id,
                                    }),
                                );
                            }
                        } else {
                            // Fallback if we somehow don't have the PK
                            return (
                                Some(ExplorerToOrchestrator::NeighborsRequest {
                                    explorer_id: self.id,
                                    current_planet_id: self.current_planet_id,
                                }),
                                None,
                            );
                        }

                        // Decide where to move or change state if everything is explored
                        let unexplored = self.knowledge.get_unexplored_planets();
                        match self.find_first_unexplored(&unexplored) {
                            None => {
                                // All planets are explored
                                self.change_state(Collecting);
                            }
                            Some(id) => {
                                self.knowledge.set_destination(Some(id));
                                let next_hop = self.knowledge.get_next_hop(self.current_planet_id);
                                return (
                                    Some(ExplorerToOrchestrator::TravelToPlanetRequest {
                                        explorer_id: self.id,
                                        current_planet_id: self.current_planet_id,
                                        dst_planet_id: next_hop,
                                    }),
                                    None,
                                );
                            }
                        }
                        (None, None)
                    }
                    Collecting => {
                        // If there is something left to collect:
                        // Check if current planet has the required resource;
                        // If not, check neighbors;
                        // If not, find another planet
                        match self.anything_left_on_the_shopping_list() {
                            None => {
                                self.change_state(StrategyState::Crafting);
                                (None, None) // Idle for one turn to run the cycle again
                            }
                            Some(list) => {
                                // check current planet
                                let current_resources = self
                                    .knowledge
                                    .get_planet_knowledge(self.current_planet_id)
                                    .unwrap() // This is safe because all planets have been explored in step 1
                                    .get_resource_type();

                                // Check if current planet has any required resources
                                let mut compatible_resources = list
                                    .iter()
                                    .filter(|(resource_type, _)| {
                                        current_resources.contains(resource_type)
                                    })
                                    .map(|(resource_type, _)| *resource_type)
                                    .collect::<Vec<_>>();

                                // If there are some:
                                if !compatible_resources.is_empty() {
                                    // Request resources from current planet
                                    return (
                                        None,
                                        Some(ExplorerToPlanet::GenerateResourceRequest {
                                            explorer_id: self.id,
                                            resource: compatible_resources.pop().unwrap(), // safe because it cannot be empty
                                        }),
                                    );
                                } else {
                                    // Find another planet
                                    // Look in list what resource I need the most
                                    if let Some((&needed_resource, _)) =
                                        list.iter().max_by_key(|&(_, count)| count)
                                    {
                                        // Filter known planets that have the resource
                                        let candidates: Vec<&crate::explorers::allegory::knowledge::PlanetKnowledge> =
                                            self.knowledge
                                                .planets
                                                .iter()
                                                .filter(|p| {
                                                    p.get_resource_type().contains(&needed_resource)
                                                })
                                                .collect();

                                        // Planets of type A have the prioriy
                                        let target_planet = candidates
                                            .iter()
                                            .find(|p| {
                                                matches!(
                                                    p.get_planet_type(),
                                                    common_game::components::planet::PlanetType::A
                                                )
                                            })
                                            .or_else(|| candidates.first()); // otherwise any works

                                        if let Some(target) = target_planet {
                                            let target_id = target.get_id();
                                            self.knowledge.set_destination(Some(target_id));
                                            let next_hop =
                                                self.knowledge.get_next_hop(self.current_planet_id);

                                            // Move there
                                            (
                                                Some(
                                                    ExplorerToOrchestrator::TravelToPlanetRequest {
                                                        explorer_id: self.id,
                                                        current_planet_id: self.current_planet_id,
                                                        dst_planet_id: next_hop,
                                                    },
                                                ),
                                                None,
                                            )
                                        } else {
                                            // No planet has the needed resource - collection impossible
                                            log::warn!(
                                                "No planet found with resource {:?}",
                                                needed_resource
                                            );
                                            self.change_state(StrategyState::Failed);
                                            return (None, None);
                                        }
                                    } else {
                                        // Empty shopping list (shouldn't happen)
                                        self.change_state(StrategyState::Crafting);
                                        (None, None)
                                    }
                                }
                            }
                        }
                    }
                    Crafting => {
                        // Verify if with last turn the task is finished
                        if self.verify_win() {
                            self.change_state(Finished);
                            return (None, None);
                        }

                        // If not, :(, apply the same logic as collecting but with crafting

                        match self.anything_left_on_the_crafting_list() {
                            None => {
                                // Should never  happen (just checked) but go back to collecting
                                if self.anything_left_on_the_shopping_list().is_some() {
                                    self.change_state(Collecting);
                                }
                                (None, None)
                            }
                            Some(list) => {
                                // Check current planet first
                                for item in &list {
                                    if self.knowledge.can_planet_produce(
                                        self.current_planet_id,
                                        ResourceType::Complex(*item.0),
                                    ) {
                                        if self
                                            .knowledge
                                            .get_planet_knowledge(self.current_planet_id)
                                            .unwrap()
                                            .get_latest_cells_number()
                                            > 0
                                        {
                                            return (
                                                None,
                                                Some(ExplorerToPlanet::CombineResourceRequest {
                                                    explorer_id: self.id,
                                                    msg: self
                                                        .create_complex_request(*item.0)
                                                        .unwrap(),
                                                }), // Since all resources are collected before this state, there should always be enough not to have a None
                                            );
                                        } else {
                                        }
                                    }
                                }
                                // check current planet
                                let current_combinations = self
                                    .knowledge
                                    .get_planet_knowledge(self.current_planet_id)
                                    .unwrap()
                                    .get_combinations();

                                // Check if current planet has any required combinations
                                let mut compatible_combinations = list
                                    .iter()
                                    .filter(|(complex_type, _)| {
                                        current_combinations.contains(complex_type)
                                    })
                                    .map(|(complex_type, _)| *complex_type)
                                    .collect::<Vec<_>>();

                                if !compatible_combinations.is_empty() {
                                    let req_type = compatible_combinations.pop().unwrap();
                                    let request = match self.create_complex_request(req_type) {
                                        None => {
                                            // Should never happen
                                            self.change_state(StrategyState::Collecting);
                                            return (None, None);
                                        }
                                        Some(request) => request,
                                    };

                                    (
                                        None,
                                        Some(ExplorerToPlanet::CombineResourceRequest {
                                            explorer_id: self.id,
                                            msg: request,
                                        }),
                                    )
                                } else {
                                    // Find another planet
                                    if let Some((&needed_complex, _)) =
                                        list.iter().max_by_key(|&(_, count)| count)
                                    {
                                        let candidates: Vec<&crate::explorers::allegory::knowledge::PlanetKnowledge> =
                                            self.knowledge
                                                .planets
                                                .iter()
                                                .filter(|p| {
                                                    p.get_combinations().contains(&needed_complex)
                                                })
                                                .collect();

                                        // Priority is given to C type planets
                                        let target_planet = candidates
                                            .iter()
                                            .find(|p| {
                                                matches!(
                                                    p.get_planet_type(),
                                                    common_game::components::planet::PlanetType::C
                                                )
                                            })
                                            .or_else(|| candidates.first());

                                        if let Some(target) = target_planet {
                                            let target_id = target.get_id();
                                            self.knowledge.set_destination(Some(target_id));
                                            let next_hop =
                                                self.knowledge.get_next_hop(self.current_planet_id);

                                            (
                                                Some(
                                                    ExplorerToOrchestrator::TravelToPlanetRequest {
                                                        explorer_id: self.id,
                                                        current_planet_id: self.current_planet_id,
                                                        dst_planet_id: next_hop,
                                                    },
                                                ),
                                                None,
                                            )
                                        } else {
                                            // There is no way of achieving the goal
                                            self.change_state(Failed);
                                            (None, None)
                                        }
                                    } else {
                                        (None, None)
                                    }
                                }
                            }
                        }
                    }
                    Finished | Failed => {
                        // Should never happen, but do nothing
                        (None, None)
                    }
                }
            }
            Some(_) => {
                (
                    Some(ExplorerToOrchestrator::TravelToPlanetRequest {
                        explorer_id: self.id,
                        current_planet_id: self.current_planet_id,
                        dst_planet_id: self.knowledge.get_next_hop(self.current_planet_id), // destination is used internally
                    }),
                    None,
                )
            }
        }
    }
}

use crate::explorers::BagContent;

mod test {
    use std::collections::HashSet;
    use common_game::components::resource::{BasicResourceType, ComplexResourceType};
    use super::*;
    use crossbeam_channel::unbounded;

    pub fn create_test_explorer() -> (
        AllegoryExplorer,
        Sender<OrchestratorToExplorer>,
        Receiver<ExplorerToOrchestrator<BagContent>>,
        Sender<PlanetToExplorer>,
        Receiver<ExplorerToPlanet>,
    ) {
        let (tx_orch, rx_orch) = unbounded();
        let (tx_ex_to_orch, rx_ex_to_orch) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_ex_to_planet, rx_ex_to_planet) = unbounded();

        let explorer = AllegoryExplorer::new_complete(
            1,
            1,
            rx_orch,
            tx_ex_to_orch,
            tx_ex_to_planet,
            rx_planet,
            HashMap::new(),
        );

        (explorer, tx_orch, rx_ex_to_orch, tx_planet, rx_ex_to_planet)
    }

    #[test]
    fn test_next_step_exploring_no_knowledge() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let (orch_msg, planet_msg) = explorer.next_step(Exploring);

        // When no knowledge exists for current planet, should request neighbors
        assert!(orch_msg.is_some());
        assert!(planet_msg.is_none());

        match orch_msg {
            Some(ExplorerToOrchestrator::NeighborsRequest {
                explorer_id,
                current_planet_id,
            }) => {
                assert_eq!(explorer_id, 1);
                assert_eq!(current_planet_id, 1);
            }
            _ => panic!("Expected NeighborsRequest"),
        }
    }

    #[test]
    fn test_next_step_exploring_missing_resources() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Add planet knowledge with neighbors but no resources
        let neighbors = HashSet::from([2, 3]);
        let pk = crate::explorers::allegory::knowledge::PlanetKnowledge::new(
            1,
            common_game::components::planet::PlanetType::A,
            neighbors,
            HashSet::new(), // empty resources
            HashSet::new(),
            0,
        );
        explorer.knowledge.planets.push(pk);

        let (orch_msg, planet_msg) = explorer.next_step(Exploring);

        // Should request resource information
        assert!(orch_msg.is_none());
        assert!(planet_msg.is_some());

        match planet_msg {
            Some(ExplorerToPlanet::SupportedResourceRequest { explorer_id }) => {
                assert_eq!(explorer_id, 1);
            }
            _ => panic!("Expected SupportedResourceRequest"),
        }
    }

    #[test]
    fn test_next_step_exploring_missing_combinations() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Add planet knowledge with neighbors and resources but no combinations
        let neighbors = HashSet::from([2, 3]);
        let resources = HashSet::from([BasicResourceType::Oxygen, BasicResourceType::Silicon]);
        let pk = crate::explorers::allegory::knowledge::PlanetKnowledge::new(
            1,
            common_game::components::planet::PlanetType::A,
            neighbors,
            resources,
            HashSet::new(), // empty combinations
            0,
        );
        explorer.knowledge.planets.push(pk);

        let (orch_msg, planet_msg) = explorer.next_step(Exploring);

        // Should request combination information
        assert!(orch_msg.is_none());
        assert!(planet_msg.is_some());

        match planet_msg {
            Some(ExplorerToPlanet::SupportedCombinationRequest { explorer_id }) => {
                assert_eq!(explorer_id, 1);
            }
            _ => panic!("Expected SupportedCombinationRequest"),
        }
    }

    #[test]
    fn test_next_step_exploring_complete_knowledge_all_explored() {
        pub fn create_test_explorer() -> (
            AllegoryExplorer,
            Sender<OrchestratorToExplorer>,
            Receiver<ExplorerToOrchestrator<BagContent>>,
            Sender<PlanetToExplorer>,
            Receiver<ExplorerToPlanet>,
        ) {
            let (tx_orch, rx_orch) = unbounded();
            let (tx_ex_to_orch, rx_ex_to_orch) = unbounded();
            let (tx_planet, rx_planet) = unbounded();
            let (tx_ex_to_planet, rx_ex_to_planet) = unbounded();

            let explorer = AllegoryExplorer::new_complete(
                1,
                1,
                rx_orch,
                tx_ex_to_orch,
                tx_ex_to_planet,
                rx_planet,
                HashMap::new(),
            );

            (explorer, tx_orch, rx_ex_to_orch, tx_planet, rx_ex_to_planet)
        }
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Add complete planet knowledge and all neighbors are also known
        let neighbors = HashSet::from([2, 3]);
        let resources = HashSet::from([BasicResourceType::Oxygen, BasicResourceType::Silicon]);
        let combinations = HashSet::from([ComplexResourceType::Water]);

        let pk1 = crate::explorers::allegory::knowledge::PlanetKnowledge::new(
            1,
            common_game::components::planet::PlanetType::A,
            neighbors,
            resources,
            combinations,
            10,
        );

        let pk2 = crate::explorers::allegory::knowledge::PlanetKnowledge::new(
            2,
            common_game::components::planet::PlanetType::B,
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        let pk3 = crate::explorers::allegory::knowledge::PlanetKnowledge::new(
            3,
            common_game::components::planet::PlanetType::C,
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        explorer.knowledge.planets.push(pk1);
        explorer.knowledge.planets.push(pk2);
        explorer.knowledge.planets.push(pk3);

        let (orchestrator_message, planet_msg) =
            explorer.next_step(Exploring);

        // All planets are explored, so no action should be taken
        assert!(orchestrator_message.is_none());
        assert!(planet_msg.is_none());
    }

    #[test]
    fn test_next_step_collecting() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let (orchestrator_message, planet_msg) =
            explorer.next_step(Collecting);

        // TODO!
        assert!(orchestrator_message.is_none());
        assert!(planet_msg.is_none());
    }

    #[test]
    fn test_next_step_finished() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let (orchestrator_message, planet_msg) =
            explorer.next_step(Finished);

        // Finished state should do nothing
        assert!(orchestrator_message.is_none());
        assert!(planet_msg.is_none());
    }

    #[test]
    fn test_next_step_failed() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let (orchestrator_message, planet_msg) = explorer.next_step(Failed);

        // Failed state should do nothing
        assert!(orchestrator_message.is_none());
        assert!(planet_msg.is_none());
    }
}
