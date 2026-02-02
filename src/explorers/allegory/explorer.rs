use crate::explorers::allegory::bag::Bag;
use crate::explorers::allegory::knowledge::{ExplorerKnowledge, StrategyState};
use common_game::components::resource::{
    BasicResource, BasicResourceType, ComplexResource, ComplexResourceRequest, ComplexResourceType,
    ResourceType,
};
use common_game::protocols::{
    orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer},
    planet_explorer::{ExplorerToPlanet, PlanetToExplorer},
};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use crate::explorers::BagContent;

#[allow(dead_code)]
pub struct AllegoryExplorer {
    pub id: ID,

    // Current State
    pub current_planet_id: ID,
    pub mode: ExplorerMode,

    // Communication Channels
    pub rx_orchestrator: Receiver<OrchestratorToExplorer>,
    pub tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
    pub tx_planet: Sender<ExplorerToPlanet>,
    pub rx_planet: Receiver<PlanetToExplorer>,

    // Resources
    pub bag: Bag,
    pub bag_content: BagContent,

    // Information collection
    pub knowledge: ExplorerKnowledge,

    pub task: HashMap<ResourceType, usize>,
    pub simple_resources_task: HashMap<BasicResourceType, usize>,
}

#[allow(dead_code)]
pub enum ExplorerMode {
    Auto,
    Manual,
    Stopped,
    Killed,
    Retired
}

impl AllegoryExplorer {
    pub fn new_complete(
        id: ID,
        current_planet_id: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        tx_planet: Sender<ExplorerToPlanet>,
        rx_planet: Receiver<PlanetToExplorer>,
        task: HashMap<ResourceType, usize>,
    ) -> Self {
        AllegoryExplorer {
            id,
            current_planet_id,
            mode: ExplorerMode::Stopped,
            rx_orchestrator,
            tx_orchestrator,
            tx_planet,
            rx_planet,
            bag: Default::default(),
            bag_content: BagContent{content: HashMap::new()},
            knowledge: Default::default(),
            task,
            simple_resources_task: HashMap::new(),
        }
    }

    pub(crate) fn add_basic_to_bag(&mut self, resource: BasicResource) {
        let resource_type = resource.get_type();
        self.bag
            .basic_resources
            .entry(resource_type)
            .or_default()
            .push(resource);
    }

    pub(crate) fn add_complex_to_bag(&mut self, resource: ComplexResource) {
        let resource_type = resource.get_type();
        self.bag
            .complex_resources
            .entry(resource_type)
            .or_default()
            .push(resource);
    }

    pub(crate) fn change_state(&mut self, new_state: StrategyState) {
        self.knowledge.update_state(new_state);
    }

    /// Function to find the closest unexplored planet for mapping.
    pub(crate) fn find_first_unexplored(&self, targets: &HashSet<ID>) -> Option<ID> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parents = HashMap::new();

        queue.push_back(self.current_planet_id);
        visited.insert(self.current_planet_id);

        while let Some(curr) = queue.pop_front() {
            if let Some(pk) = self.knowledge.get_planet_knowledge(curr) {
                for &neighbor in pk.get_neighbors() {
                    // Check if neighbor is in targets (unexplored)
                    if targets.contains(&neighbor) {
                        // Reconstruct path to find the first step
                        if curr == self.current_planet_id {
                            return Some(neighbor);
                        }

                        let mut trace = curr;
                        while let Some(&p) = parents.get(&trace) {
                            if p == self.current_planet_id {
                                return Some(trace);
                            }
                            trace = p;
                        }
                        return Some(trace);
                    }

                    // Only visit neighbors that we have knowledge of
                    if !visited.contains(&neighbor)
                        && self.knowledge.get_planet_knowledge(neighbor).is_some()
                    {
                        visited.insert(neighbor);
                        parents.insert(neighbor, curr);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        None
    }

    /// Determines the resources left to harvest before moving on to crafting
    /// Uses the difference between simple_resource_task and basic resources in bag_content
    pub(crate) fn anything_left_on_the_shopping_list(
        &self,
    ) -> Option<HashMap<BasicResourceType, usize>> {
        let mut missing = HashMap::new();

        // Iterate through simple_resources_task, find what's needed and get how much there is
        for (resource_type, &needed_count) in &self.simple_resources_task {
            let have_count = self
                .bag_content
                .content
                .get(&ResourceType::Basic(*resource_type))
                .copied()
                .unwrap_or(0);
            // Get diff
            if needed_count > have_count {
                missing.insert(*resource_type, needed_count - have_count);
            }
        }

        // Return Some if there are missing resources, None otherwise
        if missing.is_empty() {
            None
        } else {
            Some(missing)
        }
    }

    /// Determines the resources left to craft
    /// Uses the difference between task and complex resources in bag_content
    pub(crate) fn anything_left_on_the_crafting_list(
        &self,
    ) -> Option<HashMap<ComplexResourceType, usize>> {
        let mut missing = HashMap::new();

        // Iterate through simple_resources_task, find what's needed and get how much there is
        for (resource_type, &needed_count) in &self.task {
            match resource_type {
                ResourceType::Basic(_) => continue,
                ResourceType::Complex(complex) => {
                    let have_count = self
                        .bag_content
                        .content
                        .get(&ResourceType::Complex(*complex))
                        .copied()
                        .unwrap_or(0);
                    // Get diff
                    if needed_count > have_count {
                        missing.insert(*complex, needed_count - have_count);
                    }
                }
            }
        }

        // Return Some if there are missing resources, None otherwise
        if missing.is_empty() {
            None
        } else {
            Some(missing)
        }
    }

    /// Checks the task and the bag content to verify if everything required for retirement was collected / crafted
    pub(crate) fn verify_win(&self) -> bool {
        // Compare each element of task with Bag; if not enough, early return false
        for (resource_type, &required_count) in &self.task {
            match resource_type {
                ResourceType::Basic(basic) => {
                    let owned_count = self
                        .bag_content
                        .content
                        .get(&ResourceType::Basic(*basic))
                        .copied()
                        .unwrap_or(0);
                    if owned_count < required_count {
                        return false;
                    }
                }
                ResourceType::Complex(complex) => {
                    let owned_count = self
                        .bag_content
                        .content
                        .get(&ResourceType::Complex(*complex))
                        .copied()
                        .unwrap_or(0);
                    if owned_count < required_count {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Produces a list of basic resource types based on the initial task
    fn complex_to_simple_list(&mut self) {
        let mut map: HashMap<BasicResourceType, usize> = HashMap::new();
        for (resource, count) in &self.task {
            match resource {
                ResourceType::Basic(basic) => {
                    *map.entry(*basic).or_insert(0) += count;
                }
                ResourceType::Complex(complex) => match complex {
                    ComplexResourceType::Water => {
                        *map.entry(BasicResourceType::Hydrogen).or_insert(0) += count;
                        *map.entry(BasicResourceType::Oxygen).or_insert(0) += count;
                    }
                    ComplexResourceType::Diamond => {
                        *map.entry(BasicResourceType::Carbon).or_insert(0) += count * 2;
                    }
                    ComplexResourceType::Life => {
                        *map.entry(BasicResourceType::Hydrogen).or_insert(0) += count;
                        *map.entry(BasicResourceType::Oxygen).or_insert(0) += count;
                        *map.entry(BasicResourceType::Carbon).or_insert(0) += count;
                    }
                    ComplexResourceType::Robot => {
                        *map.entry(BasicResourceType::Silicon).or_insert(0) += count;
                        *map.entry(BasicResourceType::Hydrogen).or_insert(0) += count;
                        *map.entry(BasicResourceType::Oxygen).or_insert(0) += count;
                        *map.entry(BasicResourceType::Carbon).or_insert(0) += count;
                    }
                    ComplexResourceType::Dolphin => {
                        *map.entry(BasicResourceType::Hydrogen).or_insert(0) += count * 2;
                        *map.entry(BasicResourceType::Oxygen).or_insert(0) += count * 2;
                        *map.entry(BasicResourceType::Carbon).or_insert(0) += count;
                    }
                    ComplexResourceType::AIPartner => {
                        *map.entry(BasicResourceType::Silicon).or_insert(0) += count;
                        *map.entry(BasicResourceType::Carbon).or_insert(0) += count * 3;
                        *map.entry(BasicResourceType::Hydrogen).or_insert(0) += count;
                        *map.entry(BasicResourceType::Oxygen).or_insert(0) += count;
                    }
                },
            }
        }
        self.simple_resources_task = map;
    }

    /// Creates a complex resource request with bag resources
    /// Fails if there are not enough resources
    pub(crate) fn create_complex_request(
        &mut self,
        resource: ComplexResourceType,
    ) -> Option<ComplexResourceRequest> {
        match resource {
            ComplexResourceType::Diamond => {
                let c1 = self.bag.get_basic_resource(BasicResourceType::Carbon);
                let c2 = self.bag.get_basic_resource(BasicResourceType::Carbon);
                if c1.is_none() || c2.is_none() {
                    // get carbon back if any
                    self.attempt_recovery_of_basic_resource(c1);
                    self.attempt_recovery_of_basic_resource(c2);
                    return None;
                }
                Some(ComplexResourceRequest::Diamond(
                    c1.unwrap().to_carbon().unwrap(),
                    c2.unwrap().to_carbon().unwrap(),
                )) // Unwraps are safe because carbon is basic and not none is checked beforehand
            }
            ComplexResourceType::Water => {
                let h = self.bag.get_basic_resource(BasicResourceType::Hydrogen);
                let o = self.bag.get_basic_resource(BasicResourceType::Oxygen);
                if h.is_none() || o.is_none() {
                    self.attempt_recovery_of_basic_resource(h);
                    self.attempt_recovery_of_basic_resource(o);
                    return None;
                }
                Some(ComplexResourceRequest::Water(
                    h.unwrap().to_hydrogen().unwrap(),
                    o.unwrap().to_oxygen().unwrap(),
                ))
            }
            ComplexResourceType::Life => {
                let w = self.bag.get_complex_resource(ComplexResourceType::Water);
                let c = self.bag.get_basic_resource(BasicResourceType::Carbon);
                if w.is_none() || c.is_none() {
                    self.attempt_recovery_of_complex_resource(w);
                    self.attempt_recovery_of_basic_resource(c);
                    return None;
                }
                Some(ComplexResourceRequest::Life(
                    w.unwrap().to_water().unwrap(),
                    c.unwrap().to_carbon().unwrap(),
                ))
            }
            ComplexResourceType::Robot => {
                let s = self.bag.get_basic_resource(BasicResourceType::Silicon);
                let l = self.bag.get_complex_resource(ComplexResourceType::Life);
                if s.is_none() || l.is_none() {
                    self.attempt_recovery_of_basic_resource(s);
                    self.attempt_recovery_of_complex_resource(l);
                    return None;
                }
                Some(ComplexResourceRequest::Robot(
                    s.unwrap().to_silicon().unwrap(),
                    l.unwrap().to_life().unwrap(),
                ))
            }
            ComplexResourceType::Dolphin => {
                let w = self.bag.get_complex_resource(ComplexResourceType::Water);
                let l = self.bag.get_complex_resource(ComplexResourceType::Life);
                if w.is_none() || l.is_none() {
                    self.attempt_recovery_of_complex_resource(w);
                    self.attempt_recovery_of_complex_resource(l);
                    return None;
                }
                Some(ComplexResourceRequest::Dolphin(
                    w.unwrap().to_water().unwrap(),
                    l.unwrap().to_life().unwrap(),
                ))
            }
            ComplexResourceType::AIPartner => {
                let r = self.bag.get_complex_resource(ComplexResourceType::Robot);
                let d = self.bag.get_complex_resource(ComplexResourceType::Diamond);
                if r.is_none() || d.is_none() {
                    self.attempt_recovery_of_complex_resource(r);
                    self.attempt_recovery_of_complex_resource(d);
                    return None;
                }
                Some(ComplexResourceRequest::AIPartner(
                    r.unwrap().to_robot().unwrap(),
                    d.unwrap().to_diamond().unwrap(),
                ))
            }
        }
    }
    /// Helper to get back some resource in case of failure
    fn attempt_recovery_of_basic_resource(&mut self, r1: Option<BasicResource>) {
        if let Some(resource) = r1 {
            self.add_basic_to_bag(resource);
        }
    }

    fn attempt_recovery_of_complex_resource(&mut self, r1: Option<ComplexResource>) {
        if let Some(resource) = r1 {
            self.add_complex_to_bag(resource);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_game::components::resource::{
        BasicResource, BasicResourceType, Diamond, Hydrogen, Life, Oxygen, Water,
    };
    use crossbeam_channel::unbounded;
    use crate::explorers::allegory::knowledge::PlanetKnowledge;

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
    fn test_new() {
        let (explorer, _, _, _, _) = create_test_explorer();
        assert_eq!(explorer.id, 1);
        assert_eq!(explorer.current_planet_id, 1);
        match explorer.mode {
            ExplorerMode::Stopped => {}
            _ => panic!("New explorer should be stopped"),
        }
    }

    #[test]
    fn test_bag_content_conversion() {
        let mut bag = Bag::default();

        // Add 2 oxygens
        for _ in 0..2 {
            let oxygen: Oxygen = unsafe { std::mem::zeroed() };
            bag.basic_resources
                .entry(BasicResourceType::Oxygen)
                .or_default()
                .push(BasicResource::Oxygen(oxygen));
        }

        let content = BagContent::from_bag(&bag);
        assert_eq!(
            content.content.get(&ResourceType::Basic(BasicResourceType::Oxygen)),
            Some(&2)
        );
        assert_eq!(
            content.content.get(&ResourceType::Basic(BasicResourceType::Hydrogen)),
            None
        );
    }

    #[test]
    fn test_supported_resource_flow_fails_due_to_missing_planet_knowledge() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let mut resources = HashSet::new();
        resources.insert(BasicResourceType::Oxygen);

        // 1. Update knowledge
        let msg_planet = PlanetToExplorer::SupportedResourceResponse {
            resource_list: resources.clone(),
        };
        explorer.handle_planet_message(msg_planet).unwrap();

        // 2. Request from orchestrator
        let res =
            explorer.handle_orchestrator_message(OrchestratorToExplorer::SupportedResourceRequest);

        // Currently expected to fail because knowledge doesn't have the planet entry and cannot create it
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            "Failed to send to orchestrator SupportedResourceResult: value unknown".to_string()
        );
    }

    #[test]
    fn test_add_basic_to_bag_single_resource() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let oxygen: Oxygen = unsafe { std::mem::zeroed() };
        let resource = BasicResource::Oxygen(oxygen);

        explorer.add_basic_to_bag(resource);

        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content.content.get(&ResourceType::Basic(BasicResourceType::Oxygen)),
            Some(&1)
        );
    }

    #[test]
    fn test_add_basic_to_bag_multiple_same_type() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        for _ in 0..3 {
            let oxygen: Oxygen = unsafe { std::mem::zeroed() };
            let resource = BasicResource::Oxygen(oxygen);
            explorer.add_basic_to_bag(resource);
        }

        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content.content.get(&ResourceType::Basic(BasicResourceType::Oxygen)),
            Some(&3)
        );
    }

    #[test]
    fn test_add_basic_to_bag_multiple_different_types() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let oxygen: Oxygen = unsafe { std::mem::zeroed() };
        explorer.add_basic_to_bag(BasicResource::Oxygen(oxygen));

        let hydrogen: Hydrogen = unsafe { std::mem::zeroed() };
        explorer.add_basic_to_bag(BasicResource::Hydrogen(hydrogen));

        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content.content.get(&ResourceType::Basic(BasicResourceType::Oxygen)),
            Some(&1)
        );
        assert_eq!(
            bag_content
                .content
                .get(&ResourceType::Basic(BasicResourceType::Hydrogen)),
            Some(&1)
        );
        assert_eq!(bag_content.content.len(), 2);
    }

    #[test]
    fn test_add_complex_to_bag_single_resource() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let water: common_game::components::resource::Water = unsafe { std::mem::zeroed() };
        let resource = ComplexResource::Water(water);

        explorer.add_complex_to_bag(resource);

        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content
                .content
                .get(&ResourceType::Complex(ComplexResourceType::Water)),
            Some(&1)
        );
    }

    #[test]
    fn test_add_complex_to_bag_multiple_same_type() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        for _ in 0..2 {
            let diamond: Diamond = unsafe { std::mem::zeroed() };
            let resource = ComplexResource::Diamond(diamond);
            explorer.add_complex_to_bag(resource);
        }

        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content
                .content
                .get(&ResourceType::Complex(ComplexResourceType::Diamond)),
            Some(&2)
        );
    }

    #[test]
    fn test_add_complex_to_bag_multiple_different_types() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let water: Water = unsafe { std::mem::zeroed() };
        explorer.add_complex_to_bag(ComplexResource::Water(water));

        let life: Life = unsafe { std::mem::zeroed() };
        explorer.add_complex_to_bag(ComplexResource::Life(life));

        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content
                .content
                .get(&ResourceType::Complex(ComplexResourceType::Water)),
            Some(&1)
        );
        assert_eq!(
            bag_content
                .content
                .get(&ResourceType::Complex(ComplexResourceType::Life)),
            Some(&1)
        );
        assert_eq!(bag_content.content.len(), 2);
    }

    #[test]
    fn test_add_basic_and_complex_to_bag() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        let oxygen: Oxygen = unsafe { std::mem::zeroed() };
        explorer.add_basic_to_bag(BasicResource::Oxygen(oxygen));

        let water: Water = unsafe { std::mem::zeroed() };
        explorer.add_complex_to_bag(ComplexResource::Water(water));

        let bag_content = BagContent::from_bag(&explorer.bag);
        assert_eq!(
            bag_content.content.get(&ResourceType::Basic(BasicResourceType::Oxygen)),
            Some(&1)
        );
        assert_eq!(
            bag_content
                .content
                .get(&ResourceType::Complex(ComplexResourceType::Water)),
            Some(&1)
        );
    }

    #[test]
    fn test_find_first_unexplored_direct_neighbor() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Setup: current planet (1) has an unexplored neighbor (2)
        let neighbors = HashSet::from([2]);
        let pk1 = PlanetKnowledge::new(
            1,
            common_game::components::planet::PlanetType::A,
            neighbors,
            HashSet::new(),
            HashSet::new(),
            0,
        );
        explorer.knowledge.planets.push(pk1);

        // Target is planet 2 (unexplored)
        let targets = HashSet::from([2]);
        let result = explorer.find_first_unexplored(&targets);

        assert_eq!(result, Some(2), "Expected to find direct neighbor 2");
    }

    #[test]
    fn test_find_first_unexplored_multi_hop() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Setup: chain of planets 1 -> 2 -> 3, where 3 is the target (unexplored)
        let pk1 = PlanetKnowledge::new(
            1,
            common_game::components::planet::PlanetType::A,
            HashSet::from([2]),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        let pk2 = PlanetKnowledge::new(
            2,
            common_game::components::planet::PlanetType::B,
            HashSet::from([1, 3]),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        explorer.knowledge.planets.push(pk1);
        explorer.knowledge.planets.push(pk2);

        // Target is planet 3 (unexplored, requires going through 2)
        let targets = HashSet::from([3]);
        let result = explorer.find_first_unexplored(&targets);

        assert_eq!(
            result,
            Some(2),
            "Expected first step to be planet 2 on path to 3"
        );
    }

    #[test]
    fn test_find_first_unexplored_no_targets() {
        let (explorer, _, _, _, _) = create_test_explorer();

        let targets = HashSet::new();
        let result = explorer.find_first_unexplored(&targets);

        assert_eq!(result, None, "Should return None when no targets provided");
    }

    #[test]
    fn test_find_first_unexplored_unreachable_target() {
        let (explorer, _, _, _, _) = create_test_explorer();

        // Targets exist but are not reachable from current planet (no knowledge of intermediate planets)
        let targets = HashSet::from([5, 6, 7]);
        let result = explorer.find_first_unexplored(&targets);

        assert_eq!(
            result, None,
            "Should return None when targets are unreachable"
        );
    }

    #[test]
    fn test_find_first_unexplored_multiple_targets_returns_closest() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Setup: planet 1 has neighbors 2 and 3
        // Planet 2 has neighbor 4
        // Targets: 3 and 4 (both unexplored)
        // Should return 3 because it's a direct neighbor (closer)
        let pk1 = PlanetKnowledge::new(
            1,
            common_game::components::planet::PlanetType::A,
            HashSet::from([2, 3]),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        let pk2 = PlanetKnowledge::new(
            2,
            common_game::components::planet::PlanetType::B,
            HashSet::from([1, 4]),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        explorer.knowledge.planets.push(pk1);
        explorer.knowledge.planets.push(pk2);

        let targets = HashSet::from([3, 4]);
        let result = explorer.find_first_unexplored(&targets);

        // Should find 3 as it's a direct neighbor (closer than 4)
        assert_eq!(
            result,
            Some(3),
            "Expected to find closest target 3 before 4"
        );
    }

    #[test]
    fn test_find_first_unexplored_with_single_neighbor() {
        let (mut explorer, _, _, _, _) = create_test_explorer();

        // Setup: current planet has one neighbor which is the target
        let pk1 = PlanetKnowledge::new(
            1,
            common_game::components::planet::PlanetType::A,
            HashSet::from([10]),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        explorer.knowledge.planets.push(pk1);

        let targets = HashSet::from([10]);
        let result = explorer.find_first_unexplored(&targets);

        assert_eq!(
            result,
            Some(10),
            "Expected to find single neighbor as target"
        );
    }
}
