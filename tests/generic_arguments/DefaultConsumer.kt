import genericargs.Defaults
fun main() {
    val box=Defaults<String>("OK")
    check(box.fallback<String>() == null)
    check(box.repeat("OK") == "OK")
    check(box.repeat(second="OK", first="wrong") == "OK")
    check(box.classifier(42) == "OK")
    check(box.nullable("OK") == null)
    check(box.sequence("OK") == "OK")
    check(box.sequence(first="wrong", second="OK") == "OK")
    println("OK")
}
