package consumer
import inference.*
fun main() {
    check(early(true).isEmpty())
    check(initialized(false).single() == "value")
    check(guarded("test") == 4)
    check(receiver("x") == 1)
    check(standard(null) == -1)
    check(matchers().size == 2)
    check(bound("") == null)
    check(standardBound("full") == "full")
    check(nullableFunction(5) == "5")
    check(nullableFunctionRef(7) == "7")
    check(absentFunction(8) == null)
    check(nullableJoined(listOf(1.toByte(), 2.toByte())) == "item item")
    println("OK")
}
