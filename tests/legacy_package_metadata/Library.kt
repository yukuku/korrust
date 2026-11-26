@file:JvmName("PortableApi")

package compatibility.library

@Target(AnnotationTarget.CLASS)
@Retention(AnnotationRetention.RUNTIME)
annotation class Marker(val label: String)

typealias Label = String

fun answer(seed: Int = 7): Int = seed

fun <T> relay(value: T): T = value

fun String.lengthPlus(delta: Int): Int = length + delta

val packageValue: Int
    get() = 11

const val PACKAGE_CONSTANT: Int = 13

@JvmName("jvmAnswer")
fun namedAnswer(value: Int): Int = value + 17
