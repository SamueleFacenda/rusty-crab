use common_game::components::resource::{
    BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, ResourceType
};
use std::collections::HashMap;

/// Struct to store the actual items
#[allow(dead_code)]
pub struct Bag {
    pub (crate) basic_resources: HashMap<BasicResourceType, Vec<BasicResource>>,
    pub (crate) complex_resources: HashMap<ComplexResourceType, Vec<ComplexResource>>,
}

impl Bag {
    /// Extracts a basic resource from bag if it is available
    pub(crate) fn get_basic_resource(
        &mut self,
        resource: BasicResourceType,
    ) -> Option<BasicResource> {
        let vec = self.basic_resources.get_mut(&resource);
        vec?.pop()
    }

    /// Extracts a complex resource from bag if it is available
    pub(crate) fn get_complex_resource(
        &mut self,
        resource: ComplexResourceType,
    ) -> Option<ComplexResource> {
        let vec = self.complex_resources.get_mut(&resource);
        vec?.pop()
    }
}
impl Default for Bag {
    fn default() -> Self {
        Bag {
            basic_resources: HashMap::new(),
            complex_resources: HashMap::new(),
        }
    }
}

// Overwritten by crate

/*
/// Struct to store the content of the bag
#[allow(dead_code)]
pub struct BagContent {
    pub(crate) basic_resources: HashMap<BasicResourceType, usize>,
    pub(crate) complex_resources: HashMap<ComplexResourceType, usize>,
}
impl Default for BagContent {
    fn default() -> Self {
        BagContent {
            basic_resources: HashMap::new(),
            complex_resources: HashMap::new(),
        }
    }
}
*/
use crate::explorers::BagContent;

#[allow(dead_code)]
impl BagContent {
    pub fn from_bag(bag: &Bag) -> Self {
        let mut content = HashMap::new();
        
        for (k, v) in &bag.basic_resources {
            content.insert(ResourceType::Basic(*k), v.len());
        }
        
        for (k, v) in &bag.complex_resources {
            content.insert(ResourceType::Complex(*k), v.len());
        }

        BagContent {
            content
        }
    }
}