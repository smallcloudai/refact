/// This is a simple struct representing a Point in 2D space
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    /// Creates a new Point with the given x and y coordinates
    ///
    /// # Arguments
    ///
    /// * `x` - The x coordinate
    /// * `y` - The y coordinate
    ///
    /// # Example
    ///
    /// ```
    /// let p = Point::new(3.0, 4.0);
    /// ```
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    /// Returns the x coordinate of the Point
    pub fn get_x(&self) -> f64 {
        self.x
    }

    /// Returns the y coordinate of the Point
    pub fn get_y(&self) -> f64 {
        self.y
    }
}