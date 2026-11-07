import anonymousprobe.*
fun main() {
    val value: Counter = counter(3)
    check(value.add(4) == 7)
    check(value.self() === value)
    check(shared() == 710)
    println("OK")
}
