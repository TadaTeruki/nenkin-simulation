use naturalneighbor::Point;

#[derive(Debug, Clone)]
pub struct Site {
    pub x: f64,
    pub y: f64,
}

impl Site {
    pub fn distance(&self, other: &Site) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn squared_distance(&self, other: &Site) -> f64 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }
}

impl From<Site> for Point {
    fn from(site: Site) -> Self {
        Point { x: site.x, y: site.y }
    }
}