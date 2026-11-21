package metadata
import consumer.ref
import consumer.nullable
import consumer.inherited
import roots.Leaf
import consumer.relayVacancies
fun main() {
    check(ref(null) == "pre:null:post")
    check(nullable(null) == "[null]")
    check(inherited(Leaf()) == 114)
    check(relayVacancies<String>(2).size == 2)
    println("OK")
}
