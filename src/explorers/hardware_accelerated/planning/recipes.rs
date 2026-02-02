use common_game::components::resource::{BasicResourceType, ComplexResourceRequest, ComplexResourceType,
                                        GenericResource, ResourceType};

pub fn get_resource_recipe(resource: &ComplexResourceType) -> (ResourceType, ResourceType) {
    match resource {
        ComplexResourceType::Water =>
            (ResourceType::Basic(BasicResourceType::Hydrogen), ResourceType::Basic(BasicResourceType::Oxygen)),
        ComplexResourceType::Diamond =>
            (ResourceType::Basic(BasicResourceType::Carbon), ResourceType::Basic(BasicResourceType::Carbon)),
        ComplexResourceType::Life =>
            (ResourceType::Complex(ComplexResourceType::Water), ResourceType::Basic(BasicResourceType::Carbon)),
        ComplexResourceType::Robot =>
            (ResourceType::Basic(BasicResourceType::Silicon), ResourceType::Complex(ComplexResourceType::Life)),
        ComplexResourceType::Dolphin =>
            (ResourceType::Complex(ComplexResourceType::Water), ResourceType::Complex(ComplexResourceType::Life)),
        ComplexResourceType::AIPartner =>
            (ResourceType::Complex(ComplexResourceType::Robot), ResourceType::Complex(ComplexResourceType::Diamond)),
    }
}

pub fn get_resource_request(
    res_type: ComplexResourceType,
    a: GenericResource,
    b: GenericResource
) -> ComplexResourceRequest {
    match res_type {
        ComplexResourceType::Water => ComplexResourceRequest::Water(a.to_hydrogen().unwrap(), b.to_oxygen().unwrap()),
        ComplexResourceType::Diamond => ComplexResourceRequest::Diamond(a.to_carbon().unwrap(), b.to_carbon().unwrap()),
        ComplexResourceType::Life => ComplexResourceRequest::Life(a.to_water().unwrap(), b.to_carbon().unwrap()),
        ComplexResourceType::Robot => ComplexResourceRequest::Robot(a.to_silicon().unwrap(), b.to_life().unwrap()),
        ComplexResourceType::Dolphin => ComplexResourceRequest::Dolphin(a.to_water().unwrap(), b.to_life().unwrap()),
        ComplexResourceType::AIPartner =>
            ComplexResourceRequest::AIPartner(a.to_robot().unwrap(), b.to_diamond().unwrap()),
    }
}
