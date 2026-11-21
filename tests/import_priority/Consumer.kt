package priority.reader
import priority.consumer.*
fun main() {
    val one = priority.one.StringBuilder(11)
    val wildcard = Wildcard("wildcard", one, one)
    check(wildcard.value == "wildcard" && wildcard.builder === one && wildcard.alias === one)
    check(Explicit("explicit", priority.two.StringBuilder(22)).value == "explicit")
    val local = priority.local.StringBuilder(33)
    check(priority.local.SamePackage("local", local).builder === local)
    check(Aliased("alias", one).value == "alias")
    check(Duplicate(one).builder === one)
    println("OK")
}
