package visibilitygenericconsumer

import visibilitygeneric.Dual

fun main() {
    val reader = Dual<Int, String>(1)
    val value = "\u0000\ud83d\ude03"
    check(reader.readAsB(Dual<String, String>(value)) === value)
    println("OK")
}
