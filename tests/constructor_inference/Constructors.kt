package constructors.consumer

import constructors.library.JavaTypes
import constructors.library.JavaTypes.Pair
import constructors.library.JavaTypes.Box
import constructors.library.JavaTypes.Bounded
import constructors.library.JavaTypes.FromCollection
import constructors.library.JavaTypes.Choice
import constructors.library.JavaTypes.Base
import constructors.library.JavaTypes.Flipped
import constructors.library.JavaTypes.Shadow
import constructors.library.JavaTypes.Dependent
import constructors.library.JavaTypes.Batch
import constructors.library.Maker
import java.util.LinkedList

class KBox<T>(val value: T)
class KDefault<T>(val value: T? = null, val tag: Int = 7)
class KCallback<T>(val transform: (T) -> Int)
class Producer<out T>(val value: T)
class Sink<in T>(value: T) { fun tag(): Int = 17 }

fun javaBox(): String = Box("java").value
fun kotlinBox(): String = KBox("kotlin").value
fun inferredBox() = KBox("metadata")
fun <T> rebox(value: T): KBox<T> = KBox(value)
fun collection(): String = FromCollection(listOf("collection")).first()
fun linked(): String = LinkedList(listOf("linked")).get(0)
fun bounded(): String = Bounded("bound").value
fun defaults(): KDefault<String> = KDefault()
fun namedDefault(): KDefault<String> = KDefault(tag = 9)
fun named(): KBox<String> = KBox(value = "named")
fun nullable(): KBox<String?> = KBox(null)
fun context(): KBox<List<String>> = KBox(emptyList())
fun contextualLambda(): KCallback<String> = KCallback { it.length }
fun covariance(): Producer<CharSequence> = Producer("cov")
fun contravariance(): Sink<String> { val value: Any = "sink"; return Sink(value) }
fun projected(): Base<Int, String> = Flipped()
fun shadowedParameter(): Shadow<Int> = Shadow("shadow")
fun dependent(): String = Dependent("dependent").value
fun dependentExpected(): Dependent<CharSequence> = Dependent("expected")
fun choice(): Int = Choice(listOf("x")).tag
fun batch(): String = Batch("one", "two").first()
fun factory(): String = Maker("factory").get()
fun shadow(KBox: (String) -> String): String = KBox("local")

fun javaPrimitive(): Int = Box(7).value
fun kotlinPrimitive(): Int = KBox(8).value
fun numericBatch(): Int = Batch(4, 5).first()

fun <T> genericLambda(): KCallback<T> = KCallback { 6 }

class KPair<A, B>(val first: A, val second: B) { init { JavaTypes.mark("K") } }
@JvmInline value class Count(val raw: Int)
fun valueClass(): Int = KBox(Count(12)).value.raw
fun javaOrder(): String {
    JavaTypes.clear()
    val pair = Pair(JavaTypes.argument("A"), JavaTypes.argument("B"))
    return JavaTypes.trace()
}
fun kotlinOrder(): String {
    JavaTypes.clear()
    val pair = KPair(second = JavaTypes.argument("B"), first = JavaTypes.argument("A"))
    return pair.first + pair.second + ":" + JavaTypes.trace()
}

fun nullableParameter(): KDefault<String> = KDefault(value = null)
fun javaNull(): Box<String> = Box(null)
