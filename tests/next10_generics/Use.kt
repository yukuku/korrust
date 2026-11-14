package generics
interface Producer<out T> { fun get(): T }
interface Consumer<in T> { fun accept(value: T): Int }
class Box<out T>(val value: T) : Producer<T> {
    override fun get(): T = value
}
class Sink<in T> : Consumer<T> {
    override fun accept(value: T): Int = 17
}
interface Transform<in I, out O> { fun apply(f: (O) -> I): (I) -> O }
interface Tag<T> { fun tag(): T }
class Marked(val name: String) : Tag<String> { override fun tag(): String = name }
class Tagged<T : Tag<String>>(val value: T) { fun text(): String = value.tag() }
interface Label { fun text(): String }
class Named(val name: String) : Label { override fun text(): String = name }
interface Bounded<out T : Label> { fun get(): T }
class Holder<out T : Label>(val value: T) : Bounded<T> {
    override fun get(): T = value
    fun text(): String = value.text()
}
interface BoundSink<in T : Label> { fun accept(value: T): String }
class NamedSink : BoundSink<Named> { override fun accept(value: Named): String = value.text() }
class OtherLabel : Label { override fun text(): String = "other" }
class Chars<T : CharSequence>(val value: T) {
    fun size(): Int = value.length
}
fun widen(box: Producer<String>): Producer<Any> = box
fun narrow(sink: Consumer<Any>): Consumer<String> = sink
fun run(): String {
    val producer: Producer<Any> = Box<String>("OK")
    val sink: Consumer<String> = Sink<Any>()
    val holder = Holder<Named>(Named("OK"))
    val bounded: Bounded<Label> = holder
    return holder.text() + bounded.get().text() + Chars<String>("abc").size() + sink.accept("x") + producer.get().toString()
}
