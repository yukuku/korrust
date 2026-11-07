package fileconsumer
import fileuses.answer
import fileuses.label
import fileuses.other
fun main() {
    check(answer() == 42)
    check(label == "value")
    check(other() == 13)
    println("OK:consumer")
}
