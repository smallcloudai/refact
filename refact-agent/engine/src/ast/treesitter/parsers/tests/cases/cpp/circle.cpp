/**
 * @brief A class representing a circle, inheriting from Shape
 */
class Circle : public Shape {
private:
    double radius; /**< The radius of the circle */

public:
    /**
     * @brief Constructor for Circle
     * @param r The radius of the circle
     */
    Circle(double r) : radius(r) {}

    /**
     * @brief Calculate the area of the circle
     * @return The area of the circle
     */
    double calculateArea() override {
        return 3.14159 * radius * radius;
    }

    /**
     * @brief Calculate the perimeter of the circle
     * @return The perimeter of the circle
     */
    double calculatePerimeter() override {
        return 2 * 3.14159 * radius;
    }
};