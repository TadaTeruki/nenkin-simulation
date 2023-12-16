use naturalneighbor::Lerpable;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
pub enum State {
    None,
    Live(usize), // usize: passed time
    Path(usize), // usize: index of parent
    Dead,
}

#[derive(Clone)]
pub struct Property {
    pub state: State,
    pub parent: Option<usize>,
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct NumericProperty {
    pub state_none: f64,
    pub state_live: f64,
    pub state_path: f64,
    pub state_dead: f64,
}

impl Lerpable for NumericProperty {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        Self {
            state_none: self.state_none.lerp(&other.state_none, t),
            state_live: self.state_live.lerp(&other.state_live, t),
            state_path: self.state_path.lerp(&other.state_path, t),
            state_dead: self.state_dead.lerp(&other.state_dead, t),
        }
    }
}

impl From<Property> for NumericProperty {
    fn from(prop: Property) -> Self {
        let mut state_none = 0.0;
        let mut state_live = 0.0;
        let mut state_path = 0.0;
        let mut state_dead = 0.0;
        match prop.state {
            State::None => state_none = 1.0,
            State::Live(_) => state_live = 1.0,
            State::Path(_) => state_path = 1.0,
            State::Dead => state_dead = 1.0,
        }
        Self {
            state_none,
            state_live,
            state_path,
            state_dead,
        }
    }
}
