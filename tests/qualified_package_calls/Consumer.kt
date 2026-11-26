package sample
class Local { fun test(value: Int): Int = value - 2 }
fun intValue(): Int = external.nested.test(7)
fun textValue(): String = external.nested.pick<String>(external.nested.test("ok"))
fun shadow(external: Local): Int = external.test(10)
