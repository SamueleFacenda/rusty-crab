use std::collections::{HashMap, HashSet};
use bevy::ecs::query::EcsAccessType::Resource;
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, ResourceType};
use common_game::utils::ID;

pub struct ExplorerKnowledge {
    galaxy: GalaxyInfo,
    planets: HashMap<ID, PlanetInfo>,
    goal: HashMap<ResourceType, i32>  // All the resources he needs to Mine&Craft
}

impl Default for ExplorerKnowledge {
    fn default() -> Self {
        let mut goal: HashMap<ResourceType, i32> = HashMap::new();
        goal.insert(ResourceType::Basic(BasicResourceType::Oxygen), 3);
        goal.insert(ResourceType::Basic(BasicResourceType::Carbon), 4);
        goal.insert(ResourceType::Basic(BasicResourceType::Hydrogen), 3);
        goal.insert(ResourceType::Basic(BasicResourceType::Silicon), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::Life), 2);
        goal.insert(ResourceType::Complex(ComplexResourceType::Water), 3);
        goal.insert(ResourceType::Complex(ComplexResourceType::Robot), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::Diamond), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::Dolphin), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::AIPartner), 1);

        ExplorerKnowledge {
            galaxy: GalaxyInfo::default(),
            planets: HashMap::new(),
            goal
        }
    }
}

impl ExplorerKnowledge {
    pub fn update_planet(&mut self, id: ID, planet_info: PlanetInfo) {
        self.planets.insert(id, planet_info);
    }

    pub fn add_bi_connection(&mut self, a: ID, b: ID) {
        self.galaxy.add_bi_connection(a, b);
    }

    pub fn remove_bi_connection(&mut self, a: ID, b: ID) {
        self.galaxy.remove_bi_connection(&a, &b);
    }

    pub fn decrease_from_goal(&mut self, resource_type: ResourceType) {
        if let Some(value) = self.goal.get_mut(&resource_type) {
            *value -= 1;
        }
    }

    pub fn goal_completed(&self) -> bool {
        self.goal[ResourceType::Complex(ComplexResourceType::AIPartner)] == 0 &&
            self.goal[ResourceType::Complex(ComplexResourceType::Dolphin)] == 0
    }

    pub fn complex_goal_completed(&self, resource_type: ComplexResourceType) -> bool {
        self.goal[ResourceType::Complex(resource_type)] == 0
    }

    pub fn basic_goal_completed(&self, resource_type: BasicResourceType) -> bool {
        self.goal[ResourceType::Basic(resource_type)] == 0
    }

}

struct PlanetInfo {
    basic_type: HashSet<BasicResourceType>,
    complex_type: Option<ComplexResourceType>,
    energy_available: i32,
    is_destroyed: bool
}
struct GalaxyInfo{
    connections: HashMap<ID, HashSet<ID>>
}

impl PlanetInfo {
    fn new(
        basic_type: HashSet<BasicResourceType>,
        complex_type: Option<ComplexResourceType>,
        energy_available: i32
    ) -> PlanetInfo {
        PlanetInfo {
            basic_type,
            complex_type,
            energy_available,
            is_destroyed: false
        }
    }

    fn update(&mut self, energy: i32) {
        self.energy_available = energy;
    }
}

impl Default for GalaxyInfo {
    fn default() -> Self {
        GalaxyInfo {
            connections: HashMap::new()
        }
    }
}

impl GalaxyInfo {
    fn add_bi_connection(&mut self, a: ID, b: ID) {
        self.connections
            .entry(a)
            .or_insert_with(HashSet::new)
            .insert(b);
        self.connections
            .entry(b)
            .or_insert_with(HashSet::new)
            .insert(a);
    }

    // Used when planets get destroyed, it removes the connection both ways
    fn remove_bi_connection(&mut self, a: &ID, b: &ID) {
        if let Some(conns) = self.connections.get_mut(a) {
            conns.remove(b);
        }
        if let Some(conns) = self.connections.get_mut(b) {
            conns.remove(a);
        }
    }
}