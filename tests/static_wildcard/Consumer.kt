package star.reader
fun main() {
    check(star.consumer.inferred() == 17)
    check(star.consumer.fromFirst() == 15)
    check(star.consumer.fromSecond() == 25)
    check(star.consumer.alias() == 103)
    check(star.consumer.local { it + 41 } == 46)
    check(star.consumer.captured() == 19)
    check(star.consumer.reference() == 21)
    check(star.consumer.nested() == 29)
    check(star.consumer.inheritedMethod() == 34)
    println("OK")
}
