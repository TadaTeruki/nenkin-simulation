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
    weights_cache: Vec<Option<CachedWeight>>,
    kdtree: ImmutableKdTree<f64, 2>,
    lifetime: Option<usize>,
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
        let interp = Interpolator::new(&sites);
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

    pub fn set_start(self, x: f64, y: f64) -> Self {
        let nearest = self.kdtree.nearest_one::<SquaredEuclidean>(&[x, y]);
        Self {
            props: self
                .props
                .iter()
                .enumerate()
                .map(|(idx, prop)| {
                    if idx == nearest.item as usize {
                        Property {
                            state: State::Live(0),
                            parent: None,
                        }
                    } else {
                        prop.clone()
                    }
                })
                .collect(),
            ..self
        }
    }

    pub fn set_lifetime(self, lifetime: usize) -> Self {
        Self {
            lifetime: Some(lifetime),
            ..self
        }
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

    #[wasm_bindgen]
    pub fn get_nearest_site(&self, x: f64, y: f64) -> Option<usize> {
        let nearest = self.kdtree.nearest_one::<SquaredEuclidean>(&[x, y]);
        Some(nearest.item as usize)
    }
    pub fn get_property(
        &mut self,
        x: f64,
        y: f64,
        cache_key: Option<usize>,
    ) -> Option<NumericProperty> {
        let site = Site { x, y };
        if let Some(key) = cache_key {
            if key >= self.weights_cache.len() {
                self.weights_cache.resize(key + 1, None);
            }
            if let Some(cache) = &self.weights_cache[key] {
                if let Some(cache) = cache {
                    let mut property: Option<NumericProperty> = None;
                    for (idx, weight) in cache {
                        let other = NumericProperty::from(self.props[*idx].clone());
                        if let Some(prop) = property {
                            property = Some(prop.add(&other.mul_scala(*weight)));
                        } else {
                            property = Some(other.mul_scala(*weight));
                        }
                    }
                    return property;
                } else {
                    return None;
                }
            }
        }
        let weights = self.interp.query_weights(site.clone());
        if let Some(key) = cache_key {
            self.weights_cache[key] = Some(weights.clone());
        }
        if let Some(weights) = weights {
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