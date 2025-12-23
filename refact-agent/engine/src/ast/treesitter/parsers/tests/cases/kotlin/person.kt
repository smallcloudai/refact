package com.example.model

import java.time.LocalDate

data class Person(
    val name: String,
    val age: Int,
    val email: String? = null,
    val birthDate: LocalDate? = null
) {
    fun isAdult(): Boolean = age >= 18
    
    fun getDisplayName(): String = name.uppercase()
    
    companion object {
        fun create(name: String, age: Int): Person {
            return Person(name, age)
        }
    }
}

interface Greetable {
    fun greet(): String
}

class Employee(
    name: String,
    age: Int,
    val department: String,
    val salary: Double
) : Person(name, age), Greetable {
    
    override fun greet(): String {
        return "Hello, I'm $name from $department department"
    }
    
    fun getAnnualSalary(): Double = salary * 12
}

enum class Department {
    ENGINEERING,
    MARKETING,
    SALES,
    HR
}

object Company {
    val name = "TechCorp"
    val employees = mutableListOf<Employee>()
    
    fun addEmployee(employee: Employee) {
        employees.add(employee)
    }
    
    fun getEmployeeCount(): Int = employees.size
}
