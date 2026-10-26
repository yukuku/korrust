package compatibility.declarations.consumer
import compatibility.declarations.*
fun main() {
    check(discardedLocal() == "12:I2")
    check(discardedAll() == "I")
    check(discardedParameters() == 10)
    check(discardedLambda() == "33:I3")
    check(repeatedDiscardParameters() == 11)
    check(discardedLoop() == "9:II33")
    println("OK")
}
