package fieldconsumer
import fieldapi.*
class Child : Protected() { fun read(): String = value }
fun main() {
    val value = Capsule("kotlin", 3, "ordinary")
    check(value.name == "kotlin" && value.derived == "kotlin!")
    value.count = 9
    value.label = "changed"
    value.ticks = 27L
    check(value.increment() == 10 && value.label == "changed" && value.ticks == 27L)
    check(value.absent == null && value.maybe == null && value.isReady)
    check(ordered() == "aabic:aabc")
    check(GenericBox("generic").item == "generic")
    check(AccessorName("field").value == "field")
    check(AccessorName("field").getValue() == "explicit")
    check(Child().read() == "protected")
    check(Shadow("shadow").value == "shadow")
    println("OK")
}
