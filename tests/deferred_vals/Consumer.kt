package deferredvalsconsumer
import deferredvals.*
fun main() {
    check(choose(true) == 3 && choose(false) == 4)
    check(nested(4) == 24)
    check(capture(true)() == 9)
    check(selectedExpression(false) == 24)
    println("deferred-consumer-ok")
}
