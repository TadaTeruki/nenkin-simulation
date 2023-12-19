use crate::{
    core::Site,
    types::{NumericProperty, Property, State},
};
use kiddo::{ImmutableKdTree, SquaredEuclidean};
use naturalneighbor::Interpolator;
use terrain_graph::undirected::UndirectedGraph;
use wasm_bindgen::prelude::*;

type CachedWeight = Option<Vec<(usize, f64)>>;
#[wasm_bindgen]
pub struct Network {
    sites: Vec<Site>,
    props: Vec<Property>,
    graph: UndirectedGraph,
    interp: Interpolator,
    kdtree: ImmutableKdTree<f64, 2>,
    weights_cache: Vec<CachedWeight>,
    lifetime: Option<usize>,
}

#[wasm_bindgen]
pub struct Weight {
    pub index: usize,
    pub weight: f64,
}

#[wasm_bindgen]
impl Weight {
    pub fn new(index: usize, weight: f64) -> Weight {
        Weight { index, weight }
    }
}

#[wasm_bindgen]
impl Network {
    pub(crate) fn new(sites: Vec<Site>, graph: UndirectedGraph) -> Option<Network> {
        let props = vec![
            Property {
                state: State::None,
                parent: None
            };
            sites.len()
        ];
        let interp: Interpolator = Interpolator::new(&sites);
        let kdtree = ImmutableKdTree::new_from_slice(
            &sites
                .iter()
                .map(|site| [site.x, site.y])
                .collect::<Vec<_>>(),
        );

        Some(Network {
            sites,
            props,
            graph,
            interp,
            weights_cache: vec![],
            kdtree,
            lifetime: None,
        })
    }

    pub fn seartch_path(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let mut current = from;
        let mut path = vec![current];
        while current != to {
            let mut min_dist_to = f64::MAX;
            let mut min_index = None;
            self.graph
                .neighbors_of(current)
                .iter()
                .for_each(|neighbor| {
                    let dist_to = self.sites[*neighbor].distance(&self.sites[to]);
                    if min_dist_to > dist_to {
                        min_dist_to = dist_to;
                        min_index = Some(*neighbor);
                    }
                });
            if let Some(min_index) = min_index {
                path.push(min_index);
                current = min_index;
            } else {
                return None;
            }
        }
        Some(path)
    }

    pub fn set_wall(&mut self, x: f64, y: f64, prev_x: f64, prev_y: f64) {
        let nearest = self.kdtree.nearest_one::<SquaredEuclidean>(&[x, y]);
        let prev_nearest = self
            .kdtree
            .nearest_one::<SquaredEuclidean>(&[prev_x, prev_y]);
        let path = self.seartch_path(prev_nearest.item as usize, nearest.item as usize);
        if let Some(path) = path {
            path.iter().for_each(|idx| {
                self.props[*idx] = Property {
                    state: State::Wall,
                    parent: None,
                };
            });
        }
    }

    pub fn set_start(&mut self, x: f64, y: f64) {
        let nearest: kiddo::NearestNeighbour<f64, u64> =
            self.kdtree.nearest_one::<SquaredEuclidean>(&[x, y]);
        self.props[nearest.item as usize] = Property {
            state: State::Live(0),
            parent: None,
        };
    }

    pub fn set_lifetime(&mut self, lifetime: usize) {
        self.lifetime = Some(lifetime);
    }

    fn find_child(&self, idx: usize) -> Option<usize> {
        let neighbors = self.graph.neighbors_of(idx);
        for neighbor_idx in neighbors {
            if let Some(neighbor_parent) = &self.props[*neighbor_idx].parent {
                if *neighbor_parent == idx {
                    return Some(*neighbor_idx);
                }
            }
        }
        None
    }

    fn calculate_next_prop(&self, idx: usize, lifetime: usize) -> Property {
        let target_state = &self.props[idx].state;
        let target_parent = &self.props[idx].parent;
        match target_state {
            State::None => {
                let target_site = &self.sites[idx];
                let neighbors = self
                    .graph
                    .neighbors_of(idx)
                    .iter()
                    .filter(|&neighbor_idx| {
                        if let State::Live(_) = self.props[*neighbor_idx].state {
                            return true;
                        }
                        false
                    })
                    .collect::<Vec<_>>();

                if neighbors.is_empty() {
                    // remain
                    return Property {
                        state: target_state.clone(),
                        parent: *target_parent,
                    };
                };
                // the most nearest neighbor will be parent
                let parent_idx = (0..neighbors.len()).fold(None, |acc: Option<&usize>, i| {
                    if let Some(acc) = acc {
                        let distance = target_site.distance(&self.sites[*neighbors[i]]);
                        let acc_distance = target_site.distance(&self.sites[*acc]);
                        if distance > acc_distance {
                            return Some(acc);
                        }
                    }
                    Some(neighbors[i])
                });
                // refrain
                Property {
                    state: State::Live(0),
                    parent: parent_idx.copied(),
                }
            }
            State::Live(age) => {
                let new_age = age + 1;
                if new_age >= lifetime {
                    if let Some(child_idx) = self.find_child(idx) {
                        // keep alive as a path
                        return Property {
                            state: State::Path(child_idx),
                            parent: *target_parent,
                        };
                    }
                    // die
                    return Property {
                        state: State::Dead,
                        parent: None,
                    };
                }
                // age
                Property {
                    state: State::Live(new_age),
                    parent: *target_parent,
                }
            }
            State::Path(child_idx) => {
                if let Some(child_parent) = &self.props[*child_idx].parent {
                    if *child_parent == idx {
                        // keep itself as a path
                        return Property {
                            state: target_state.clone(),
                            parent: *target_parent,
                        };
                    }
                }
                if let Some(child_idx) = self.find_child(idx) {
                    // path with another child
                    return Property {
                        state: State::Path(child_idx),
                        parent: *target_parent,
                    };
                }
                // if there is no child, die
                Property {
                    state: State::Dead,
                    parent: None,
                }
            }
            State::Dead => Property {
                state: State::Dead,
                parent: None,
            },
            State::Wall => Property {
                state: State::Wall,
                parent: None,
            },
        }
    }

    pub fn iterate(&mut self) -> bool {
        let lifetime = if let Some(lifetime) = self.lifetime {
            lifetime
        } else {
            return false;
        };

        let props: Vec<Property> = (0..self.props.len())
            .map(|idx| self.calculate_next_prop(idx, lifetime))
            .collect::<Vec<_>>();

        self.props = props;

        true
    }

    pub fn get_nearest_site(&self, x: f64, y: f64) -> Option<usize> {
        let nearest = self.kdtree.nearest_one::<SquaredEuclidean>(&[x, y]);
        Some(nearest.item as usize)
    }

    pub fn add_cache(&mut self, x: f64, y: f64) -> usize {
        let site = Site { x, y };
        let weights = self.interp.query_weights(site);
        self.weights_cache.push(weights);
        self.weights_cache.len() - 1
    }

    pub fn get_property(&self, key: usize) -> Option<NumericProperty> {
        if let Some(weights) = &self.weights_cache[key] {
            return weights
                .iter()
                .map(|(i, w)| NumericProperty::from(self.props[*i].clone()).mul_scala(*w))
                .fold(None, |acc, x| {
                    if let Some(acc) = acc {
                        Some(acc.add(&x))
                    } else {
                        Some(x)
                    }
                });
        } else {
            None
        }
    }
}
