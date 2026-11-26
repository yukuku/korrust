package constructorvarargsconsumer
import constructorvarargs.*
fun main(args: Array<String>) {
    check(Strings("c").values.size == 0)
    check(Strings("c", "a", "bb").values[1] == "bb")
    val input = arrayOf("a", "bb")
    val spread = Strings("c", *input)
    input[0] = "changed"
    check(spread.values[0] == "a")
    check(Ints(1, 2, 3).values.size == 3)
    check(Generic<String>("x", "yy").values[1] == "yy")
    check(Generic<Int>(1, 2).values[1] == 2)
    check(Default(values = intArrayOf(3, 4)).prefix == 7)
    val mutable = Mutable("a")
    mutable.values = arrayOf("replacement")
    check(mutable.values[0] == "replacement")
    check(Bare(1, 2).count == 2)
    println("constructor-consumer-ok")
}
