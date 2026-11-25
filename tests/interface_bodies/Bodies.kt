package interfacebodies

interface Label { fun text(): String }
class Name(val value: String): Label { override fun text(): String = value }

interface Generic<C> {
    fun own(): Generic<C> = this
    fun echo(value: C): C = value
    fun <T> identity(value: T): T = value
    fun <T: Label> bounded(value: T): T = value
    fun <T: Label> label(value: T): String = value.text()
    fun <T> consume(value: T, context: C): C = echo(context)
    fun <T> relay(value: T): T = identity(value)
    fun <T> closure(value: T): () -> T = { value }
}
interface Child<C>: Generic<C>
interface StringChild: Generic<String>
class ChildText: Child<String>
class ChildNumber: Child<Int>
class Specialized: StringChild
interface Bounded<C: Label> { fun same(value: C): C = value }
class BoundedImpl: Bounded<Name>
interface Property<C> {
    val value: C
    val copy: C get() = value
}
class PropertyText(override val value: String): Property<String>
class Text: Generic<String>
class IntDefault: Generic<Int>
class OverrideDefault: Generic<String> {
    override fun <R> identity(value: R): R = value
    override fun echo(value: String): String = value + "!"
}

interface Ordinary {
    fun <T> identity(value: T): T = value
    fun <T: Label> bounded(value: T): T = value
    fun <T: Label> label(value: T): String = value.text()
    fun <T> relay(value: T): T = identity(value)
}
class Plain: Ordinary
object Named: Ordinary { fun own(): String = "named" }
object Empty: Ordinary
fun anonymous(): Ordinary = object: Ordinary {}
interface Refined: Ordinary {
    override fun <R> identity(value: R): R = value
}
class RefinedImpl: Refined

fun run(): String {
    val text: Generic<String> = Text()
    val number: Generic<Int> = IntDefault()
    val ordinary: Ordinary = Plain()
    val refined: Ordinary = RefinedImpl()
    return text.echo("A") + text.identity("B") + text.bounded(Name("C")).text() + text.consume(4, "D") + text.relay("E") + number.echo(6).toString() + ordinary.identity("F") + ordinary.bounded(Name("G")).text() + ordinary.relay("H") + refined.identity("I") + OverrideDefault().echo("J")
}
