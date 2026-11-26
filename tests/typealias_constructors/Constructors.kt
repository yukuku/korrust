package aliases.consumer

import aliases.library.JavaTypes
import aliases.library.JavaTypes.Pair
import aliases.library.JavaTypes.Box
import aliases.library.JavaTypes.Bounded
import aliases.library.JavaTypes.Shadow as JavaShadow
import aliases.library.JavaTypes.Dependent as JavaDependent
import aliases.library.Imported
import aliases.library.Chained

class KBox<T>(val value: T)
class KDefault<T>(val value: T? = null)
class KCallback<T>(val callback: (T) -> Int)
class KPair<A, B>(val first: A, val second: B) { init { JavaTypes.mark("K") } }
typealias Local<T> = KBox<T>
private typealias PrivateLocal<T> = KBox<T>
typealias Swap<A, B> = Pair<B, A>
typealias Fixed<T> = Pair<String, T>
typealias Known<T> = Pair<List<String>, T>
typealias Same<T> = Pair<T, T>
typealias Nested<T> = Box<List<T>>
typealias Default<T> = KDefault<T>
typealias Callback<T> = KCallback<T>
typealias Bound<T> = Bounded<T>
typealias Shadow<T> = JavaShadow<T>
typealias Collision<T> = JavaDependent<T>
typealias Nullable<T> = Box<T>?
typealias Named<A, B> = KPair<B, A>
typealias Unused<T> = Box<String>
typealias Projected<T> = Box<out T>

fun local() = Local("local")
fun privateLocal(): String = PrivateLocal("private").value
fun swap() = Swap(8, "swap")
fun fixed() = Fixed("fixed", 9)
fun known() = Known(emptyList(), "known")
fun repeated() = Same("one", "two")
fun nested() = Nested(listOf("nested"))
fun defaults(): Default<String> = Default()
fun nullDefault(): Default<String> = Default(value = null)
fun callback(): Callback<String> = Callback { it.length }
fun imported() = Imported("imported")
fun chained() = Chained("chained")
fun bound() = Bound("bound")
fun nullable(): Nullable<String> = Nullable("nullable")
fun <T> generic(value: T): Local<T> = Local(value)
fun callableShadow(Local: (String) -> String): String = Local("callable")
fun standard(attrs: Map<String, String>) = LinkedHashMap(attrs)
fun evaluation(): String {
    JavaTypes.clear()
    val pair = Named(second = JavaTypes.argument("A"), first = JavaTypes.argument("B"))
    return pair.first + pair.second + ":" + JavaTypes.trace()
}
