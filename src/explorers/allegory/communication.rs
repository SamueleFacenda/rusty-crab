use crate::explorers::BagContent;
use common_game::components::resource::{
    BasicResourceType, ComplexResourceRequest, GenericResource,
};
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator::{
    CurrentPlanetResult, KillExplorerResult, MovedToPlanetResult, ResetExplorerAIResult,
    StartExplorerAIResult, StopExplorerAIResult,
};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crate::explorers::allegory::explorer::{AllegoryExplorer, ExplorerMode};
use std::collections::HashSet;
use crate::explorers::allegory::logging::{emit_error, emit_info, emit_warning};

impl AllegoryExplorer {
    pub(crate) fn handle_orchestrator_message(
        &mut self,
        msg: OrchestratorToExplorer,
    ) -> Result<(), String> {
        match msg {
            OrchestratorToExplorer::StartExplorerAI => {
                self.mode = ExplorerMode::Auto;
                match self.tx_orchestrator.send(StartExplorerAIResult {
                    explorer_id: self.id,
                }) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                        Err("Failed to send StartExplorerAIResult to orchestrator".to_string())
                    }
                }
            }
            OrchestratorToExplorer::ResetExplorerAI => {
                // Erase the knowledge, keep the bag
                // Is this actually what should happen?
                // Keep modes auto, killed and stopped; set to auto if manual
                if let ExplorerMode::Manual = self.mode {
                    self.mode = ExplorerMode::Auto;
                }

                // Reset knowledge
                self.knowledge = Default::default();
                match self.tx_orchestrator.send(ResetExplorerAIResult {
                    explorer_id: self.id,
                }) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                        Err("Failed to send ResetExplorerAIResult to orchestrator".to_string())
                    }
                }
            }
            OrchestratorToExplorer::KillExplorer => {
                self.mode = ExplorerMode::Killed;
                match self.tx_orchestrator.send(KillExplorerResult {
                    explorer_id: self.id,
                }) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                        Err("Failed to send KillExplorerResult to orchestrator".to_string())
                    }
                }
            }
            OrchestratorToExplorer::BagContentRequest => {
                // If killed, do not answer.
                if matches!(self.mode, ExplorerMode::Killed) {
                    return Ok(());
                }

                match self.tx_orchestrator.send(ExplorerToOrchestrator::BagContentResponse {
                    explorer_id: self.id,
                    bag_content: BagContent::from_bag(&self.bag),
                }) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                        Err("Failed to send BagContentResponse to orchestrator".to_string())
                    }
                }
            },
            OrchestratorToExplorer::StopExplorerAI => {
                self.mode = ExplorerMode::Stopped;
                match self.tx_orchestrator.send(StopExplorerAIResult {
                    explorer_id: self.id,
                }) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                        Err("Failed to send StopExplorerAIResult to orchestrator".to_string())
                    }
                }
            }
            OrchestratorToExplorer::MoveToPlanet {
                sender_to_new_planet,
                planet_id,
            } => {
                // Update planet information for new planet
                self.current_planet_id = planet_id;
                match sender_to_new_planet {
                    None => {}
                    Some(sender) => {
                        self.tx_planet = sender;
                    }
                }
                match self.tx_orchestrator.send(MovedToPlanetResult {
                    explorer_id: self.id,
                    planet_id: self.current_planet_id,
                }) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                         emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                        Err("Failed to send MovedToPlanetResult to orchestrator".to_string())
                    }
                }
            }
            OrchestratorToExplorer::CurrentPlanetRequest => {
                match self.tx_orchestrator.send(CurrentPlanetResult {
                    explorer_id: self.id,
                    // In no case the explorer can answer this question before being placed in a planet
                    planet_id: self.current_planet_id,
                }) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                         emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                        Err("Failed to send CurrentPlanetResult to orchestrator".to_string())
                    }
                }
            }
            OrchestratorToExplorer::SupportedResourceRequest => {// Send request to planet to get available combinations
                self.send_to_planet(ExplorerToPlanet::SupportedResourceRequest {
                    explorer_id: self.id,
                });

                // Wait for planet response
                match self.rx_planet.recv() {
                    Ok(PlanetToExplorer::SupportedResourceResponse { resource_list }) => {
                        // Update knowledge with received combinations
                        self.knowledge.update_planet_resource(self.current_planet_id, resource_list.clone());

                        // Forward result to orchestrator
                        match self.tx_orchestrator.send(ExplorerToOrchestrator::SupportedResourceResult {
                            explorer_id: self.id,
                            supported_resources: resource_list,
                        }) {
                            Ok(()) => Ok(()),
                            Err(e) => {
                                emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                                Err("Failed to send SupportedResourceResult to orchestrator".to_string())
                            }
                        }
                    }
                    Ok(other) => {
                        let _ = self.handle_planet_message(other);
                        Err("Unexpected planet response".to_string())
                    }
                    Err(e) => Err(format!("Failed to receive from planet: {e}")),
                }
            }
            OrchestratorToExplorer::SupportedCombinationRequest => {
                // Send request to planet to get available combinations
                self.send_to_planet(ExplorerToPlanet::SupportedCombinationRequest {
                    explorer_id: self.id,
                });
                
                // Wait for planet response
                match self.rx_planet.recv() {
                    Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) => {
                        // Update knowledge with received combinations
                        self.knowledge.update_planet_combinations(self.current_planet_id, combination_list.clone());
                        
                        // Forward result to orchestrator
                        match self.tx_orchestrator.send(ExplorerToOrchestrator::SupportedCombinationResult {
                            explorer_id: self.id,
                            combination_list,
                        }) {
                            Ok(()) => Ok(()),
                            Err(e) => {
                                emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                                Err("Failed to send SupportedCombinationResult to orchestrator".to_string())
                            }
                        }
                    }
                    Ok(other) => {
                        let _ = self.handle_planet_message(other);
                        Err("Unexpected planet response".to_string())
                    }
                    Err(e) => Err(format!("Failed to receive from planet: {e}")),
                }
            }
            OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                // Send request to planet
                self.send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: self.id,
                    resource: to_generate,
                });
                
                // Wait for planet response
                match self.rx_planet.recv() {
                    Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
                        let result = match resource {
                            Some(res) => {
                                self.add_basic_to_bag(res);
                                Ok(())
                            }
                            None => {
                                emit_warning(self.id, format!("Planet {} failed to generate a resource. ", self.current_planet_id));
                                Ok(()) // Returning an OK anyways to prevent a soft error from causing an interruption
                            },
                        };
                        
                        // Forward result to orchestrator
                        match self.tx_orchestrator.send(ExplorerToOrchestrator::GenerateResourceResponse {
                            explorer_id: self.id,
                            generated: result.clone(),
                        }) {
                            Ok(()) => Ok(()),
                            Err(e) => {
                                emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
                                Err("Failed to send GenerateResourceResponse to orchestrator".to_string())
                            }
                        }
                    }
                    Ok(other) => {
                        // Unexpected planet message
                        let _ = self.handle_planet_message(other);
                        Err("Unexpected planet response".to_string())
                    }
                    Err(e) => Err(format!("Failed to receive from planet: {e}")),
                }
            }
            OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                // Send request to planet
                let complex_message = self.create_complex_request(to_generate);
                let msg = match complex_message {
                    None => {
                        emit_warning(self.id, format!("Recoverable error: failed to generate a resource request for resource {to_generate:?}. "));
                        return Ok(())
                    }
                    Some(msg) => {msg}
                };
                self.send_to_planet(ExplorerToPlanet::CombineResourceRequest {
                    explorer_id: self.id,
                    msg,
                });

                // Wait for planet response
                match self.rx_planet.recv() {
                    Ok(PlanetToExplorer::CombineResourceResponse { complex_response }) => {
                        let result = match complex_response {
                            Ok(res) => {
                                self.add_complex_to_bag(res);
                                Ok(())
                            }
                            Err((str, res1, res2)) => {
                                emit_warning(self.id, format!("Recoverable error: planet {} failed to generate a resource: {str} ", self.current_planet_id));
                                match res1{
                                    GenericResource::BasicResources(basic) => {self.add_basic_to_bag(basic);}
                                    GenericResource::ComplexResources(complex) => {self.add_complex_to_bag(complex)}
                                }
                                match res2{
                                    GenericResource::BasicResources(basic) => {self.add_basic_to_bag(basic);}
                                    GenericResource::ComplexResources(complex) => {self.add_complex_to_bag(complex)}
                                }
                                Err(format!("Planet {} failed to generate a resource: {} ", self.current_planet_id, str)) // Returning an OK anyways to prevent a soft error from causing an interruption
                            },
                        };

                        // Forward result to orchestrator
                        match self.tx_orchestrator.send(ExplorerToOrchestrator::CombineResourceResponse {
                            explorer_id: self.id,
                            generated: result.clone(),
                        }) {
                            Ok(()) => Ok(()),
                            Err(e) => {
                                emit_error(self.id, format!("Failed to send CombineResourceResponse to orchestrator: {e}"));
                                Err("Failed to send GenerateResourceResponse to orchestrator".to_string())
                            }
                        }
                    }
                    Ok(other) => {
                        // Unexpected planet message
                        let _ = self.handle_planet_message(other);
                        Err("Unexpected planet response".to_string())
                    }
                    Err(e) => Err(format!("Failed to receive from planet: {e}")),
                }
            }
            OrchestratorToExplorer::NeighborsResponse { neighbors } => {
                self.knowledge.update_neighbors(self.current_planet_id, HashSet::from_iter(neighbors));
                Ok(())
            },
        }
    }

    pub(crate) fn handle_planet_message(&mut self, msg: PlanetToExplorer) -> Result<(), String> {
        match msg {
            PlanetToExplorer::SupportedResourceResponse { resource_list } => {
                self.knowledge
                    .update_planet_resource(self.current_planet_id, resource_list);
                Ok(())
            }
            PlanetToExplorer::SupportedCombinationResponse { combination_list } => {
                self.knowledge
                    .update_planet_combinations(self.current_planet_id, combination_list);
                Ok(())
            }
            PlanetToExplorer::GenerateResourceResponse { resource } => match resource {
                Some(res) => {
                    self.add_basic_to_bag(res);
                    Ok(())
                }
                None =>{
                    emit_warning(self.id, format!("Recoverable error: planet {} failed to generate a resource. ", self.current_planet_id));
                    Ok(())
                    // Used to be:
                    // Err(format!(
                    //     "Planet {} failed to generate resource",
                    //     self.current_planet_id
                    // ))
                }
            },
            PlanetToExplorer::CombineResourceResponse { complex_response } => {
                match complex_response {
                    Ok(complex_resource) => {
                        self.add_complex_to_bag(complex_resource);
                        Ok(())
                    }
                    Err((error_msg, resource1, resource2)) => {
                        // Add the resources back to the bag
                        match resource1 {
                            GenericResource::BasicResources(r) => self.add_basic_to_bag(r),
                            GenericResource::ComplexResources(r) => self.add_complex_to_bag(r),
                        }
                        match resource2 {
                            GenericResource::BasicResources(r) => self.add_basic_to_bag(r),
                            GenericResource::ComplexResources(r) => self.add_complex_to_bag(r),
                        }
                        Err(format!(
                            "Planet {} failed to combine resources: {error_msg}",
                            self.current_planet_id
                        ))
                    }
                }
            }
            PlanetToExplorer::AvailableEnergyCellResponse { available_cells } => {
                self.knowledge
                    .add_cell(self.current_planet_id, available_cells);
                Ok(())
            }
            PlanetToExplorer::Stopped => {
                self.knowledge.update_killed_planet(self.current_planet_id);
                Ok(())
            }
        }
    }
    // Helper functions
    pub(crate) fn send_to_orchestrator(&self, msg: ExplorerToOrchestrator<BagContent>) {
        if let Err(e) = self.tx_orchestrator.send(msg) {
             emit_error(self.id, format!("Failed to send to orchestrator: {e}"));
        }
    }

    pub(crate) fn send_to_planet(&self, msg: ExplorerToPlanet) {
        if let Err(e) = &self.tx_planet.send(msg) {
             emit_error(self.id, format!("Failed to send to planet: {e}"));
        }
    }

    /// Collection of information for any planet: neighbors, cells, resources
    pub(crate) fn query_planet(&self) {
        self.send_to_orchestrator(
            ExplorerToOrchestrator::NeighborsRequest {
                explorer_id: self.id,
                current_planet_id: self.current_planet_id,
            });
        self.send_to_planet(
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: ( self.id ) }
        );
        self.send_to_planet(
            ExplorerToPlanet::SupportedResourceRequest {
                explorer_id: self.id,
            });
        self.send_to_planet(ExplorerToPlanet::SupportedCombinationRequest {
                explorer_id: self.id,
            });
    }

    pub(crate) fn request_resource_generation(&mut self, resource: BasicResourceType) {
        self.send_to_planet(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id: self.id,
            resource,
        });
    }

    pub(crate) fn request_resource_combination(
        &mut self,
        to_generate: ComplexResourceRequest,
    ) -> Result<(), String> {
        match &self
            .tx_planet
            .send(ExplorerToPlanet::CombineResourceRequest {
                explorer_id: self.id,
                msg: to_generate,
            }) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!(
                "Planet {} failed to generate resource: {}",
                self.id,
                e
            )),
        }
    }
    
    pub (crate) fn move_to_planet(&mut self, planet_id: ID) -> Result<(), String> {
        self.send_to_orchestrator(ExplorerToOrchestrator::TravelToPlanetRequest {
            explorer_id: self.id,
            current_planet_id: self.current_planet_id,
            dst_planet_id: planet_id,
        });

        // Block wait for response
        loop {
            // Need to handle both orchestrator (for MoveToPlanet or other requests) 
            // We ignore rx_planet messages during travel as we are "in transit" or "departing"
             match self.rx_orchestrator.recv() { 
                 Ok(msg) => {
                     match msg {
                        OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet, planet_id: new_id } => {
                             if new_id == planet_id {
                                 self.current_planet_id = new_id;
                                 if let Some(sender) = sender_to_new_planet {
                                     self.tx_planet = sender;
                                 }
                                 emit_info(self.id, format!("Arrived at planet {new_id}"));
                                 self.send_to_orchestrator(ExplorerToOrchestrator::MovedToPlanetResult {
                                     explorer_id: self.id,
                                     planet_id: self.current_planet_id,
                                 });
                                 return Ok(());
                             } else {
                                 // This might happen if we get a MoveToPlanet for a previous request? Or logic error.
                                 // But let's accept it as a valid move anyway.
                                  self.current_planet_id = new_id;
                                 if let Some(sender) = sender_to_new_planet {
                                     self.tx_planet = sender;
                                 }
                                 self.send_to_orchestrator(ExplorerToOrchestrator::MovedToPlanetResult {
                                     explorer_id: self.id,
                                     planet_id: self.current_planet_id,
                                 });
                                 return Err(format!("Unexpectedly moved to planet {new_id} instead of {planet_id}"));
                             }
                        }
                        OrchestratorToExplorer::BagContentRequest => { 
                            // not handling this here, done at the end of each turn
                        }
                        _ => {
                            // Delegate other messages
                            self.handle_orchestrator_message(msg)?;
                        }
                     }
                 }
                 Err(e) => return Err(format!("Orchestrator channel closed: {e}")),
             }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_game::components::resource::{
        BasicResource, BasicResourceType, ComplexResourceType, Hydrogen, Oxygen, ResourceType
    };
    use crossbeam_channel::{Receiver, Sender, unbounded};
    use crate::explorers::Explorer;

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

        let explorer = AllegoryExplorer::new(
            1,
            1,
            rx_orch,
            tx_ex_to_orch,
            tx_ex_to_planet,
            rx_planet,
        );

        (explorer, tx_orch, rx_ex_to_orch, tx_planet, rx_ex_to_planet)
    }
    #[test]
    fn test_handle_orchestrator_message_start_ai() {
        let (mut explorer, _, rx_ex_to_orch, _, _) = create_test_explorer();

        explorer
            .handle_orchestrator_message(OrchestratorToExplorer::StartExplorerAI)
            .unwrap();

        match explorer.mode {
            ExplorerMode::Auto => {}
            _ => panic!("Mode should be Auto"),
        }

        let msg = rx_ex_to_orch.try_recv().expect("Should receive message");
        match msg {
            ExplorerToOrchestrator::StartExplorerAIResult { explorer_id } => {
                assert_eq!(explorer_id, 1);
            }
            _ => panic!("Unexpected message"),
        }
    }

    #[test]
    fn test_handle_orchestrator_message_kill() {
        let (mut explorer, _, rx_ex_to_orch, _, _) = create_test_explorer();

        explorer
            .handle_orchestrator_message(OrchestratorToExplorer::KillExplorer)
            .unwrap();

        match explorer.mode {
            ExplorerMode::Killed => {}
            _ => panic!("Mode should be Killed"),
        }

        let msg = rx_ex_to_orch.try_recv().expect("Should receive message");
        match msg {
            ExplorerToOrchestrator::KillExplorerResult { explorer_id } => {
                assert_eq!(explorer_id, 1);
            }
            _ => panic!("Unexpected message"),
        }
    }

    #[test]
    fn test_handle_orchestrator_message_stop() {
        let (mut explorer, _, rx_ex_to_orch, _, _) = create_test_explorer();

        // Start first so we can check if it stops
        explorer
            .handle_orchestrator_message(OrchestratorToExplorer::StartExplorerAI)
            .unwrap();
        // Consume start response
        rx_ex_to_orch.recv().unwrap();

        explorer
            .handle_orchestrator_message(OrchestratorToExplorer::StopExplorerAI)
            .unwrap();

        match explorer.mode {
            ExplorerMode::Stopped => {}
            _ => panic!("Mode should be Stopped"),
        }

        let msg = rx_ex_to_orch.try_recv().expect("Should receive message");
        match msg {
            ExplorerToOrchestrator::StopExplorerAIResult { explorer_id } => {
                assert_eq!(explorer_id, 1);
            }
            _ => panic!("Unexpected message"),
        }
    }

    #[test]
    fn test_handle_orchestrator_message_current_planet() {
        let (mut explorer, _, rx_ex_to_orch, _, _) = create_test_explorer();

        explorer
            .handle_orchestrator_message(OrchestratorToExplorer::CurrentPlanetRequest)
            .unwrap();

        let msg = rx_ex_to_orch.try_recv().expect("Should receive message");
        match msg {
            ExplorerToOrchestrator::CurrentPlanetResult {
                explorer_id,
                planet_id,
            } => {
                assert_eq!(explorer_id, 1);
                assert_eq!(planet_id, 1);
            }
            _ => panic!("Unexpected message"),
        }
    }

    #[test]
    fn test_handle_planet_message_generate_resource() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Use unsafe to create the opaque struct Oxygen
        let oxygen: Oxygen = unsafe { std::mem::zeroed() }; // Way found to generate resources
        let resource = BasicResource::Oxygen(oxygen);

        let msg = PlanetToExplorer::GenerateResourceResponse {
            resource: Some(resource),
        };

        explorer.handle_planet_message(msg).unwrap();

        // Verify resource is in bag
        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content.content.get(&ResourceType::Basic(BasicResourceType::Oxygen)),
            Some(&1)
        );
    }

    #[test]
    fn test_handle_orchestrator_message_supported_combination() {
        let (mut explorer, _, rx_ex_to_orch, tx_planet, rx_ex_to_planet) = create_test_explorer();

        // Spawn a thread to handle the message and planet response
        let handle = std::thread::spawn(move || {
            explorer.handle_orchestrator_message(OrchestratorToExplorer::SupportedCombinationRequest)
        });

        // Planet should receive the combination request
        let planet_msg = rx_ex_to_planet.recv_timeout(std::time::Duration::from_secs(1))
            .expect("Should receive planet message");
        match planet_msg {
            ExplorerToPlanet::SupportedCombinationRequest { explorer_id } => {
                assert_eq!(explorer_id, 1);
            }
            _ => panic!("Expected SupportedCombinationRequest"),
        }

        // Simulate planet response
        let combinations = HashSet::from([ComplexResourceType::Water, ComplexResourceType::Diamond]);
        tx_planet.send(PlanetToExplorer::SupportedCombinationResponse {
            combination_list: combinations.clone(),
        }).unwrap();

        // Wait for explorer to finish processing
        handle.join().unwrap().unwrap();

        // Orchestrator should receive the result
        let orch_msg = rx_ex_to_orch.try_recv().expect("Should receive orchestrator message");
        match orch_msg {
            ExplorerToOrchestrator::SupportedCombinationResult { explorer_id, combination_list } => {
                assert_eq!(explorer_id, 1);
                assert_eq!(combination_list, combinations);
            }
            _ => panic!("Expected SupportedCombinationResult"),
        }
    }

    #[test]
    fn test_handle_orchestrator_message_generate_resource_success() {
        let (mut explorer, _, rx_ex_to_orch, tx_planet, rx_ex_to_planet) = create_test_explorer();

        // Spawn a thread to handle the message
        let handle = std::thread::spawn(move || {
            explorer.handle_orchestrator_message(OrchestratorToExplorer::GenerateResourceRequest {
                to_generate: BasicResourceType::Hydrogen,
            })
        });

        // Planet should receive the generate request
        let planet_msg = rx_ex_to_planet.recv_timeout(std::time::Duration::from_secs(1))
            .expect("Should receive planet message");
        match planet_msg {
            ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource } => {
                assert_eq!(explorer_id, 1);
                assert_eq!(resource, BasicResourceType::Hydrogen);
            }
            _ => panic!("Expected GenerateResourceRequest"),
        }

        // Simulate successful planet response
        // Usual trick for resources
        let hydrogen: Hydrogen = unsafe { std::mem::zeroed() };
        let resource = BasicResource::Hydrogen(hydrogen);
        tx_planet.send(PlanetToExplorer::GenerateResourceResponse {
            resource: Some(resource),
        }).unwrap();

        // Wait for explorer to finish processing
        let result = handle.join().unwrap();
        assert!(result.is_ok());

        // Orchestrator should receive success response
        let orch_msg = rx_ex_to_orch.try_recv().expect("Should receive orchestrator message");
        match orch_msg {
            ExplorerToOrchestrator::GenerateResourceResponse { explorer_id, generated } => {
                assert_eq!(explorer_id, 1);
                assert!(generated.is_ok());
            }
            _ => panic!("Expected GenerateResourceResponse"),
        }
    }

    #[test]
    fn test_handle_orchestrator_message_generate_resource_failure() {
        let (mut explorer, _, rx_ex_to_orch, tx_planet, rx_ex_to_planet) = create_test_explorer();

        // Spawn a thread to handle the message
        let handle = std::thread::spawn(move || {
            explorer.handle_orchestrator_message(OrchestratorToExplorer::GenerateResourceRequest {
                to_generate: BasicResourceType::Oxygen,
            })
        });

        // Planet should receive the generate request
        let _ = rx_ex_to_planet.recv_timeout(std::time::Duration::from_secs(1))
            .expect("Should receive planet message");

        // Simulate failed planet response (no resource generated)
        tx_planet.send(PlanetToExplorer::GenerateResourceResponse {
            resource: None,
        }).unwrap();

        // Wait for explorer to finish processing
        let result = handle.join().unwrap();
        assert!(result.is_ok());

        // Orchestrator should receive success (but resource was not actually generated)
        let orch_msg = rx_ex_to_orch.try_recv().expect("Should receive orchestrator message");
        match orch_msg {
            ExplorerToOrchestrator::GenerateResourceResponse { explorer_id, generated } => {
                assert_eq!(explorer_id, 1);
                // The current implementation returns Ok even on failure
                assert!(generated.is_ok());
            }
            _ => panic!("Expected GenerateResourceResponse"),
        }
    }

    #[test]
    fn test_handle_orchestrator_message_combine_resource() {
        let (mut explorer, _, rx_ex_to_orch, _, _) = create_test_explorer();

        // Currently returns not implemented error
        let result = explorer.handle_orchestrator_message(OrchestratorToExplorer::CombineResourceRequest {
            to_generate: ComplexResourceType::Water,
        });

        assert!(result.is_ok()); // Handler succeeds but sends error to orchestrator

        // Orchestrator should receive error response
        let orch_msg = rx_ex_to_orch.try_recv().expect("Should receive orchestrator message");
        match orch_msg {
            ExplorerToOrchestrator::CombineResourceResponse { explorer_id, generated } => {
                assert_eq!(explorer_id, 1);
                assert!(generated.is_err());
                assert_eq!(generated.unwrap_err(), "CombineResourceRequest not implemented");
            }
            _ => panic!("Expected CombineResourceResponse"),
        }
    }
}
