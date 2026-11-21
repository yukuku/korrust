package consumer
fun main() {
    check(trailing.sourceDefault() == "source2!")
    check(trailing.compiledNullable() == "8")
    check(trailing.joined(listOf(1.toByte(), 255.toByte())) == "01 ff")
    println("OK")
}
