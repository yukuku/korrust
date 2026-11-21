@file:OptIn(kotlin.contracts.ExperimentalContracts::class)
package inference.dependency
import kotlin.contracts.contract
fun exists(value: String?): Boolean {
    contract { returns(true) implies (value != null) }
    return value != null
}
fun absent(value: String?): Boolean {
    contract { returns(false) implies (value != null) }
    return value == null
}
fun String?.missing(): Boolean {
    contract { returns(false) implies (this@missing != null) }
    return this == null
}
fun marker(value: String?): String? {
    contract { returns(null) implies (value != null) }
    return if (value != null) null else "absent"
}
fun stringValue(value: Any?): Boolean {
    contract { returns(true) implies (value is String) }
    return value is String
}
fun pair(a: String?, b: String?): Boolean {
    contract { returns(true) implies (a != null && b != null) }
    return a != null && b != null
}
fun either(a: String?, b: String?): Boolean {
    contract { returns(true) implies (a != null || b != null) }
    return a != null || b != null
}
fun isNullOrEmpty(value: String?): Boolean = value == null
fun <C, R> C.choose(useDefault: Boolean, defaultValue: () -> R): R where C : R =
    if (useDefault) defaultValue() else this
fun <T> String.created(): Array<T> = throw IllegalArgumentException(this)
fun selected(value: String?, mode: String): Boolean {
    contract { returns(true) implies (value != null) }
    return mode.length >= 0 && value != null
}
fun selected(value: String?, mode: Int): Boolean = value == null && mode >= 0

fun nullableTruth(value: String?): Boolean? {
    contract { returns(true) implies (value != null) }
    return if (value != null) true else null
}
fun nullableFalse(value: String?): Boolean? {
    contract { returns(false) implies (value != null) }
    return if (value != null) false else null
}
inline fun <reified T> vacancies(size: Int): Array<T?> = arrayOfNulls<T>(size)
fun <T, R> mapMaybe(value: T, transform: ((T) -> R)?): R? = transform?.invoke(value)

class FalseEquivalent {
    override fun equals(other: Any?): Boolean = other == false
    override fun hashCode(): Int = 1237
}
class TrueEquivalent {
    override fun equals(other: Any?): Boolean = other == true
    override fun hashCode(): Int = 1231
}
fun anyFalse(value: String?): Any {
    contract { returns(false) implies (value != null) }
    return if (value == null) FalseEquivalent() else false
}
fun anyTrue(value: String?): Any {
    contract { returns(true) implies (value != null) }
    return if (value == null) TrueEquivalent() else true
}
