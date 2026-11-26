package ranked

open class Parent
class Child : Parent()
fun <T : Child, R> T.choose(call: (T) -> R): String = "child:" + call(this)
fun <T : Parent, R> T.choose(call: (T) -> R): String = "parent:" + call(this)

fun <A : Child, B : Parent> crossed(first: A, second: B, block: (A, B) -> Int): Int =
    block(first, second)
fun <A : Parent, B : Child> crossed(first: A, second: B, block: (A, B) -> Int): Int =
    block(first, second)
