import genericargs.Varargs
import genericargs.Word
fun main() {
    val box=Varargs<String>("saved")
    val values=arrayOf("OK", "wrong")
    check(box.first("OK", "wrong") == "OK")
    check(box.first(7,8) == 7)
    check(box.packed(*values) !== values)
    check(box.first(*values) == "OK")
    check(box.first(items=values) == "OK")
    check(box.choose("OK") == "OK")
    check(box.choose(first="wrong", items=values) == "OK")
    check(box.forward("OK") == "OK")
    check(box.forward("wrong", *values) == "OK")
    check(box.count("ignored", "A", "B") == 2)
    check(box.label(Word("OK")) == "OK")
    check(box.classifier(1, "OK") == "OK")
    check(box.primitive("ignored", 7, 8) == 7)
    println("OK")
}
