package consumer
import iteration.*
fun main() {
    var total = 0
    for (n in Sequence(4)) { total += n }
    for (n in SourceBox(4)) { total += n }
    for (n in Edge(2)..<Edge(6)) { total += n }
    for (n in Endpoint(2)..<Endpoint(6)) { total += n }
    if (total != 24) throw AssertionError(total)
    println("consumer-ok")
}
