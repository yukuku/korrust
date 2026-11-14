package consumer
import next10.shorts.*
fun main() {
    val minimum: Short = contextual()
    val boxed: Short? = boxed(minimum)
    val values: ShortArray = pack(minimum, 32767)
    check(total(values) == -1L)
    check(unbox(boxed) == minimum)
    check(spread(values).size == 4)
    check(Holder(minimum, values).scalar == minimum)
    check(shortSum(minimum, minimum) == -65536)
    check(mixed(2147483647, 1L) == 2147483648L)
    val nullable: ShortArray? = nullable(values)
    check(nullable === values)
    check(factoryAlias()[0] == minimum)
    println("OK")
}
