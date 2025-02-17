/**
 * Represents a simple model of a Person.
 */
public class Person {

    // Fields
    private String name;
    private int age;

    /**
     * Constructs a new Person with the specified name and age.
     *
     * @param name the name of the person
     * @param age  the age of the person
     */
    public Person(String name, int age) {
        this.name = name;
        this.age = age;
    }
    
    public abstract void a();
    
    /**
     * Gets the name of the person.
     *
     * @return the name of the person
     */
    public String getName() {
        return name;
    }

    /**
     * Sets the name of the person.
     *
     * @param name the new name of the person
     */
    public void setName(String name) {
        this.name = name;
    }

    /**
     * Gets the age of the person.
     *
     * @return the age of the person
     */
    public int getAge() {
        return age;
    }

    /**
     * Sets the age of the person.
     *
     * @param age the new age of the person
     */
    public void setAge(int age) {
        this.age = age;
    }

    /**
     * Returns a string representation of the person.
     *
     * @return a string representation of the person
     */
    @Override
    public String toString() {
        return "Person{name='" + name + "', age=" + age + "}";
    }

    public static void main(String[] args) {
        Person person = new Person("John Doe", 30);
        System.out.println(person);
    }
}