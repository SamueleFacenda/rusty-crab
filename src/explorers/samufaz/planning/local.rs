use std::collections::HashMap;
use std::rc::Rc;

use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};

use super::{GlobalTask, get_resource_recipe};
use crate::explorers::samufaz::Bag;
use crate::explorers::samufaz::planning::local::LocalTask::{Generate, Produce};
use crate::explorers::samufaz::planning::local::TaskTree::Leaf;

#[derive(Clone, Debug)]
pub(crate) enum LocalTask {
    Generate(BasicResourceType),
    Produce(ComplexResourceType)
}

enum TaskTree {
    None,
    Leaf(BasicResourceType),
    Node(ComplexResourceType, Rc<TaskTree>, Rc<TaskTree>) // Task, Left Subtree, Right Subtree
}

pub(crate) struct LocalPlanner;

impl LocalPlanner {
    pub fn get_execution_plan(task: &GlobalTask, bag: &Bag) -> Vec<LocalTask> {
        let complete_plan = Self::get_tree_plan_for_task(&Produce(task.resource));
        let mut reserved: HashMap<ResourceType, usize> = HashMap::new(); // Count resources reserved in bag
        let pruned_plan = Self::prune_plan(&Rc::new(complete_plan), bag, &mut reserved, true);

        Self::sorted_topsort(pruned_plan)
    }

    fn get_tree_plan_for_task(task: &LocalTask) -> TaskTree {
        match task {
            Generate(res_type) => Leaf(*res_type),
            Produce(complex_type) => {
                let (a, b) = get_resource_recipe(*complex_type);
                let plan_a = Self::get_tree_plan_for_resource(a);
                let plan_b = Self::get_tree_plan_for_resource(b);
                TaskTree::Node(*complex_type, Rc::new(plan_a), Rc::new(plan_b))
            }
        }
    }

    fn get_tree_plan_for_resource(res_type: ResourceType) -> TaskTree {
        match res_type {
            ResourceType::Basic(basic_type) => Self::get_tree_plan_for_task(&Generate(basic_type)),
            ResourceType::Complex(complex_type) => Self::get_tree_plan_for_task(&Produce(complex_type))
        }
    }

    fn prune_plan(
        tree: &Rc<TaskTree>,
        bag: &Bag,
        reserved: &mut HashMap<ResourceType, usize>,
        is_first: bool
    ) -> TaskTree {
        match &**tree {
            TaskTree::None => TaskTree::None, // Should not happen
            Leaf(res_type) => {
                let res_type_enum = ResourceType::Basic(*res_type);
                let available_count = bag.res.get(&res_type_enum).map_or(0, Vec::len);
                let reserved_count = *reserved.get(&res_type_enum).unwrap_or(&0);
                if available_count > reserved_count && !is_first {
                    reserved.entry(res_type_enum).and_modify(|e| *e += 1).or_insert(1);
                    TaskTree::None // Resource already available, no need to generate
                } else {
                    Leaf(*res_type) // Need to generate this resource
                }
            }
            TaskTree::Node(complex_type, left, right) => {
                let complex_type_enum = ResourceType::Complex(*complex_type);
                let available_count = bag.res.get(&complex_type_enum).map_or(0, Vec::len);
                let reserved_count = *reserved.get(&complex_type_enum).unwrap_or(&0);
                if available_count > reserved_count && !is_first {
                    reserved.entry(complex_type_enum).and_modify(|e| *e += 1).or_insert(1);
                    TaskTree::None // Both resources are available, no need to produce
                } else {
                    // Recursively prune left and right subtrees
                    TaskTree::Node(
                        *complex_type,
                        Rc::new(Self::prune_plan(left, bag, reserved, false)),
                        Rc::new(Self::prune_plan(right, bag, reserved, false))
                    )
                }
            }
        }
    }

    // Sorts tasks on the same topological level per type to optimize execution
    fn sorted_topsort(tree: TaskTree) -> Vec<LocalTask> {
        let mut levels: Vec<Vec<Rc<TaskTree>>> = vec![vec![Rc::new(tree)]];
        let mut current_level = 0;
        while !levels[current_level].is_empty() {
            let mut next_level: Vec<Rc<TaskTree>> = vec![];
            for node in &levels[current_level] {
                if let TaskTree::Node(_, left, right) = &**node {
                    next_level.push(Rc::clone(left));
                    next_level.push(Rc::clone(right));
                }
            }
            levels.push(next_level);
            current_level += 1;
        }

        levels.reverse();

        levels
            .into_iter()
            .flat_map(|level| {
                let mut mapped = level
                    .into_iter()
                    .filter_map(|node| match &*node {
                        TaskTree::Leaf(res_type) => Some(Generate(*res_type)),
                        TaskTree::Node(complex_type, _, _) => Some(Produce(*complex_type)),
                        TaskTree::None => None
                    })
                    .collect::<Vec<LocalTask>>();
                mapped.sort_by_key(|a| match a {
                    Generate(res_type) => *res_type as u8,
                    Produce(complex_type) => *complex_type as u8
                });
                mapped
            })
            .collect()
    }
}
