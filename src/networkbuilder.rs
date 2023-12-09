use rand::{rngs::StdRng, Rng, SeedableRng};
use terrain_graph::undirected::UndirectedGraph;
use voronoice::{BoundingBox, VoronoiBuilder};
use wasm_bindgen::prelude::*;

use crate::{core::Site, network::Network};

#[wasm_bindgen]
pub struct NetworkBuilder {
    sites: Vec<Site>,
    bound_max: Site,
}

#[wasm_bindgen]
impl NetworkBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(num: u32, bound_x: f64, bound_y: f64) -> NetworkBuilder {
        let mut rng: StdRng = SeedableRng::from_seed([0u8; 32]);
        let sites = (0..num)
            .map(|_| {
                let x = rng.gen_range(0.0..bound_x);
                let y = rng.gen_range(0.0..bound_y);
                Site { x, y }
            })
            .collect::<Vec<Site>>();

        NetworkBuilder {
            sites,
            bound_max: Site {
                x: bound_x,
                y: bound_y,
            },
        }
    }

    pub fn add_edge_sites(self, edge_num_x: Option<usize>, edge_num_y: Option<usize>) -> Self {
        let corners = [
            Site { x: 0., y: 0. },
            Site {
                x: 0.,
                y: self.bound_max.y,
            },
            Site {
                x: self.bound_max.x,
                y: self.bound_max.y,
            },
            Site {
                x: self.bound_max.x,
                y: 0.,
            },
        ];
        let edge_sites = corners
            .iter()
            .enumerate()
            .flat_map(|(i, corner)| {
                let next = &corners[(i + 1) % corners.len()];
                let edge_num = if i % 2 == 1 {
                    edge_num_x.unwrap_or(
                        (self.sites.len() as f64 / self.bound_max.y * self.bound_max.x).sqrt()
                            as usize,
                    )
                } else {
                    edge_num_y.unwrap_or(
                        (self.sites.len() as f64 / self.bound_max.x * self.bound_max.y).sqrt()
                            as usize,
                    )
                };
                let mut edge_sites = Vec::with_capacity(edge_num);
                for j in 0..edge_num {
                    let t = j as f64 / edge_num as f64;
                    let point = Site {
                        x: corner.x * (1.0 - t) + next.x * t,
                        y: corner.y * (1.0 - t) + next.y * t,
                    };
                    edge_sites.push(point);
                }
                edge_sites
            })
            .collect::<Vec<_>>();

        Self {
            sites: self.sites.into_iter().chain(edge_sites).collect::<Vec<_>>(),
            ..self
        }
    }

    pub fn relaxate_sites(self, times: usize) -> Option<NetworkBuilder> {
        if times == 0 {
            return Some(self);
        }

        let voronoi_opt = VoronoiBuilder::default()
            .set_sites(
                self.sites
                    .iter()
                    .map(|s| voronoice::Point { x: s.x, y: s.y })
                    .collect(),
            )
            .set_bounding_box(BoundingBox::new(
                voronoice::Point {
                    x: self.bound_max.x / 2.0,
                    y: self.bound_max.y / 2.0,
                },
                self.bound_max.x,
                self.bound_max.y,
            ))
            .set_lloyd_relaxation_iterations(times)
            .build();

        if let Some(voronoi) = voronoi_opt {
            return Some(Self {
                sites: voronoi
                    .sites()
                    .iter()
                    .map(|s| Site { x: s.x, y: s.y })
                    .collect::<Vec<_>>(),
                ..self
            });
        } else {
            None
        }
    }

    pub fn build(self) -> Option<Network> {
        let voronoi_opt = VoronoiBuilder::default()
            .set_sites(
                self.sites
                    .iter()
                    .map(|s| voronoice::Point { x: s.x, y: s.y })
                    .collect(),
            )
            .set_bounding_box(BoundingBox::new(
                voronoice::Point {
                    x: self.bound_max.x / 2.0,
                    y: self.bound_max.y / 2.0,
                },
                self.bound_max.x,
                self.bound_max.y,
            ))
            .build();

        if let Some(voronoi) = voronoi_opt {
            let sites = voronoi
                .sites()
                .iter()
                .map(|s| Site { x: s.x, y: s.y })
                .collect::<Vec<Site>>();

            let triangulation = voronoi.triangulation();

            let graph: UndirectedGraph = {
                let mut graph: UndirectedGraph = UndirectedGraph::new(sites.len());
                for triangle in triangulation.triangles.chunks_exact(3) {
                    let (a, b, c) = (triangle[0], triangle[1], triangle[2]);

                    if a < b {
                        graph.add_edge(a, b);
                    }
                    if b < c {
                        graph.add_edge(b, c);
                    }
                    if c < a {
                        graph.add_edge(c, a);
                    }
                }
                graph
            };
            Network::new(sites, graph)
        } else {
            None
        }
    }
}
