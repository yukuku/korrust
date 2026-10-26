package compatibility.friend
import compatibility.visibility.InternalObject
import compatibility.visibility.InternalClass
import compatibility.visibility.moduleOnly

fun main() {
    check(moduleOnly(2) == 13)
    check(InternalObject.decorate(3) == 10)
    check(InternalClass().internalValue(6) == 25)
    println("friend-ok")
}
