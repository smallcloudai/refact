package com.example

import kotlin.math.PI
import java.util.*

fun main() {
    val name = "Kotlin"
    val version = 1.8
    println("Hello, $name version $version!")
    
    val numbers = listOf(1, 2, 3, 4, 5)
    val doubled = numbers.map { it * 2 }
    
    val person = Person("Alice", 30)
    person.greet()
    
    val calculator = Calculator()
    val result = calculator.add(10, 20)
    println("Result: $result")
}

class Person(val name: String, val age: Int) {
    fun greet() {
        println("Hello, I'm $name and I'm $age years old")
    }
    
    fun isAdult(): Boolean = age >= 18
}

class Calculator {
    fun add(a: Int, b: Int): Int = a + b
    fun subtract(a: Int, b: Int): Int = a - b
    fun multiply(a: Int, b: Int): Int = a * b
    fun divide(a: Int, b: Int): Double = a.toDouble() / b.toDouble()
}
