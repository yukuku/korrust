package consumer
import fieldmeta.dependency.Choice
fun main() {
    check(fieldmeta.first() == Choice.FIRST)
    check(fieldmeta.route(Choice.SECOND) == 2)
    println("OK")
}
