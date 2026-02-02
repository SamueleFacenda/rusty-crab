use std::collections::HashMap;

use common_game::components::resource::{BasicResource, ComplexResource, ComplexResourceType, GenericResource,
                                        ResourceType};

use super::get_resource_recipe;
use crate::explorers::BagContent;

#[derive(Debug)]
pub(super) struct Bag {
    pub(super) res: HashMap<ResourceType, Vec<GenericResource>>
}

impl Bag {
    pub fn insert_basic(&mut self, resource: BasicResource) {
        self.res
            .entry(ResourceType::Basic(resource.get_type()))
            .or_default()
            .push(GenericResource::BasicResources(resource));
    }

    pub fn insert_complex(&mut self, resource: ComplexResource) {
        self.res
            .entry(ResourceType::Complex(resource.get_type()))
            .or_default()
            .push(GenericResource::ComplexResources(resource));
    }

    pub fn get_recipe_ingredients(
        &mut self,
        resource_type: ComplexResourceType
    ) -> Option<(GenericResource, GenericResource)> {
        let (a, b) = get_resource_recipe(&resource_type);
        let res_a = self.res.entry(a).or_default();
        if res_a.is_empty() {
            return None;
        }
        let a_resource = res_a.pop().unwrap();
        let res_b = self.res.entry(b).or_default();
        if res_b.is_empty() {
            // Not enough resources to combine, put back a_resource
            self.res.entry(a).or_default().push(a_resource);
            return None;
        }
        let b_resource = res_b.pop().unwrap();
        Some((a_resource, b_resource))
    }

    pub fn to_bag_content(&self) -> BagContent {
        let mut content = BagContent::default();
        for resources in self.res.values() {
            for resource in resources {
                *content.content.entry(resource.get_type()).or_default() += 1;
            }
        }
        content
    }
}
