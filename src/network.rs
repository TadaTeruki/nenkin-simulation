use crate::core::Site;
use naturalneighbor::{Interpolator, Lerpable};
use rand::{rngs::StdRng, Rng, SeedableRng};
use terrain_graph::undirected::UndirectedGraph;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Property {
    pub score: f64,
}

impl Lerpable for Property {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        Property {
            score: self.score * (1.0 - t) + other.score * t,
        }
    }
}

#[wasm_bindgen]
pub struct Network {
    sites: Vec<Site>,
    props: Vec<Property>,
    graph: UndirectedGraph,
    interp: Interpolator,
}

#[wasm_bindgen]
impl Network {
    pub(crate) fn new(sites: Vec<Site>, graph: UndirectedGraph) -> Network {
        let mut rng: StdRng = SeedableRng::from_seed([0u8; 32]);
        let props = (0..sites.len())
            .map(|_| Property {
                score: rng.gen_range(0.0..1.0),
            })
            .collect();
        let interp = Interpolator::new(&sites);
        Network {
            sites,
            props,
            graph,
            interp,
        }
    }

    pub fn iterate(self) -> Self {
        self
    }

    pub fn get_property(&self, x: f64, y: f64) -> Option<Property> {
        let site = Site { x, y };
        self.interp.interpolate(&self.props, site)
    }
}
