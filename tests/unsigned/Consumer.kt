package unsignedconsumer
import unsignedcases.*

fun main() {
    check(uintMax() == 4294967295u)
    check(ulongMax() == 18446744073709551615uL)
    check(uintAdd(uintMax(), 1u) == 0u)
    check(ulongDivide(ulongMax(), 2uL) == 9223372036854775807uL)
    check(uintElvis(null) == 7u)
    check(uintGeneric(0xffff_ffffu) == 0xffff_ffffu)
    check(ulongGeneric(ulongMax()) == ulongMax())
    check(uintList(12u) == listOf(12u))
    val pair = ScalarPair(uintMax(), ulongMax())
    val changed = pair.copy(second = 9uL)
    check(changed.first == uintMax())
    check(changed.second == 9uL)
    check(ScalarBox(12u, 15uL).first == 12u)
    check(lambdaCapture(uintMax())() == 0u)
    check(inferredAdd(uintMax(), 1u) == 0u)
    check(inferredConversion(ulongMax()) == -1)
    check(extensionCall(uintMax()) == 0u)
    println("OK 15")
}
