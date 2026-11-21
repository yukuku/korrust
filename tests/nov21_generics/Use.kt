package methodgenerics

interface Label { fun text(): String }
class Named(val textValue: String): Label { override fun text(): String = textValue }
class Generic<C>(val seed: C) {
    constructor(seed: C, ignored: Int): this(seed)
    fun <T> echo(value: T): T = value
    fun <T: Label> bounded(value: T): T = value
    fun <T: Label> text(value: T): String = value.text()
    fun <T> choose(value: T): C = seed
    fun <T, S> second(first: T, second: S): S = second
    fun <T> first(values: List<T>): T = values[0]
    fun <T> transform(value: T, block: (T) -> String): String = block(value)
}
interface GenericApi<C> {
    fun <T: Label> identity(value: T): T
    fun <T> choose(value: T): C
}
object Helpers {
    @JvmStatic fun <T: Label> bounded(value: T): T = value
    fun <T> echo(value: T): T = value
}
fun through(api: GenericApi<String>, value: Named): Named = api.identity(value)
fun chooseThrough(api: GenericApi<String>): String = api.choose(7)
fun localCalls(): String {
    val generic = Generic<String>("seed")
    val named = Named("OK")
    return generic.bounded(named).text() + generic.first(listOf("!")) + Helpers.bounded(named).text()
}

class Cov<out T>(val value: T)
class Con<in T> { fun accept(value: T): Int = 7 }
class Inv<T>(val value: T)
open class Open
class Final
fun p1(value: Cov<Any>): Int = 1
fun p2(value: Cov<Final>): Int = 2
fun p3(value: Cov<Open>): Int = 3
fun p4(value: Con<Any>): Int = 4
fun p5(value: Con<Final>): Int = 5
fun p6(value: Cov<out Any>): Int = 6
fun p7(value: Cov<out Final>): Int = 7
fun p8(value: Con<in Final>): Int = 8
fun p9(value: Inv<Cov<Open>>): Int = 9
fun p10(value: Cov<Con<Final>>): Int = 10
fun p11(value: (Cov<Open>) -> Cov<Open>): Int = 11
fun p12(value: Cov<(Open) -> Open>): Int = 12
fun <T> p13(value: Cov<T>): Int = 13
fun p14(value: Array<Cov<Open>>): Int = 14
fun p15(value: (Any) -> Any): Int = 15
fun r1(): Cov<Open> = Cov<Open>(Open())
fun r2(): Cov<out Open> = Cov<Open>(Open())
fun r3(): Con<in Final> = Con<Final>()
fun r4(): Inv<Cov<Open>> = Inv<Cov<Open>>(Cov<Open>(Open()))
fun r5(): (Cov<Open>) -> Cov<Open> = { value -> value }
class WildMethods(val input: Cov<Any>) {
    fun take(value: Con<Final>): Int = 17
    fun result(): Cov<out Open> = Cov<Open>(Open())
}
interface WildApi { fun take(value: Cov<Open>): Int; val result: Cov<out Open> }
class WildDefaults {
    @JvmOverloads fun take(value: Cov<Any>, ignored: Int = 0): Int = 18
    companion object {
        @JvmStatic fun takeStatic(value: Cov<Any>): Int = 19
    }
}
interface WildDefaultApi { fun take(value: Cov<Open>): Int = 20 }
class WildDefaultImpl: WildDefaultApi
