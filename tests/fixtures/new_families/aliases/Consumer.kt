package aliasconsumer

import aliasdep.Box
import aliasdep.Bounded
import newfamilies.aliases.*

fun main() {
    val rows: Rows<String> = listOf("a", "b")
    val nullable: MaybeRows<String> = null
    val transform: Transformer<String> = { it ?: "fallback" }
    val converter: Converter<Int, String> = { "n=$it" }
    val formatter: Format<String> = { number -> this + number }
    val slots: Slots<String> = arrayOf(null, "tail")
    val nested: Nested<String> = listOf(listOf(null))
    val ignored: IgnoredChain<Int> = "ignored"
    val optional: OptionalTransform<String> = { it + "!" }
    val box: Boxed<String> = Box("box")
    val bounded: Constrained<String> = Bounded("bound")
    check(count(rows) == 2)
    check(nullableCount(nullable) == -1)
    check(apply(null, transform) == "fallback")
    check(convert(42, converter) == "n=42")
    check(format("R", formatter) == "R3")
    check(swap(slots) == null && slots[0] == "changed")
    check(firstNested(nested) == null)
    check(ignored(ignored) == "ignored")
    check(optional(optional) == "OK!")
    check(boxed(box) === box && boxed(box).value == "box")
    check(constrained(bounded) === bounded && constrained(bounded).value == "bound")
    check(Holder(transform, rows).transform(null) == "fallback")
    val triple: TripleRows<String> = listOf(listOf(listOf("triple")))
    val mutableNested: MutableRows<String> = listOf(mutableListOf("mutable"))
    val mutableOuter: MutableOuterRows<String> = mutableListOf(listOf("outer"))
    val nullableNested: NullableRows<String> = listOf(null)
    val functions: FunctionRows<Int> = listOf({ value -> "n=$value" })
    val boxes: BoxRows<String> = listOf(box)
    val arrays: ArrayRows<String> = listOf(arrayOf("array"))
    check(countTriple(triple) == 1 && countMutableNested(mutableNested) == 1)
    check(countMutableOuter(mutableOuter) == 1 && countNullableNested(nullableNested) == 1)
    check(countFunctions(functions) == 1 && countBoxRows(boxes) == 1 && countArrayRows(arrays) == 1)
    check(genericFirst(rows) == "a")
    check(genericApply(null, transform) == "fallback")
    println("OK")
}
