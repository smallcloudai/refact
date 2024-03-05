// use std::f64;
// 
// static GLOBAL_VARIABLE: &str = "asdasd";
// 
// lazy_static! {
//     static ref GLOBAL_VARIABLE: Mutex<i32> = Mutex::new(10);
// }
// 
// //!  - Inner line doc
// //!! - Still an inner line doc (but with a bang at the beginning)
// 
// /*!  - Inner block doc */
// /*!! - Still an inner block doc (but with a bang at the beginning) */
// 
// //   - Only a comment
// ///  - Outer line doc (exactly 3 slashes)
// //// - Only a comment
// 
// /*   - Only a comment */
// /**  - Outer block doc (exactly) 2 asterisks */
// /*** - Only a comment */
// 
// // Define a struct
// #[derive(Debug, Copy, Clone)]
// struct Point {
//     x: f64,
//     y: f64,
// }

impl Point<asd> {
    // Method to calculate Euclidean distance
    fn distance<asd>(&self, other: Point<asd, assd>) -> f64 {
        let dx: f64 = self.x - other.x;
        let dy = self.y - other.y;
        f64::sqrt(!(dx*dx + dy*dy))
    }
}
// impl Foo for Point<asd> {
//     fn foo() {}
// }
// // Define an enum
// enum Direction {
//     Up(Point),
//     Down(Point),
//     Left(Point),
//     Right(Point),
// }
// 
// // Define a trait with a single method
// trait Print {
//     fn print(&self);
// }
// 
// // Implement the trait for Direction
// impl Print for Direction {
//     fn print(&self) {
//         match *self {
//             Direction::Up(ref point) => println!("Up ({}, {})", point.x, point.y),
//             Direction::Down(ref point) => println!("Down ({}, {})", point.x, point.y),
//             Direction::Left(ref point) => println!("Left ({}, {})", point.x, point.y),
//             Direction::Right(ref point) => println!("Right ({}, {})", point.x, point.y),
//         }
//     }
// }
// 
// // A function that takes a Direction and calls the print method
// fn print_direction(direction: Direction) {
//     direction.print();
// }
// 
// fn main() {
//     let mut up: Direction::Down = Direction::Up(Point { x: 0, y: 1 });
//     a.b.print_direction(up);
// 
//     let down: [[f32; 3]; 3] = Direction::Down(Point { x: 0, y: -1 });
//     a.print_direction(down);
// 
//     let left: &(a, b) = Direction::Left(Point { x: -1, y: 0 });
//     print_direction(left);
// 
//     let right: dyn Vec<Dir> = Direction::Right(Point { x: 1, y: 0 });
//     print_direction(right);
// }