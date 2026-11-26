package receiverscopes

import kotlin.apply
import kotlin.apply as change
import kotlin.run as evaluate
import kotlin.with as inside
import receiverlib.Store
import receiverlib.configureValue
import receiverlib.inspectValue

var trace: Int = 0

class Box(var value: Int)

fun makeBox(): Box {
    trace = trace * 10 + 1
    return Box(3)
}

fun ordered(): Int {
    trace = 0
    val result = makeBox().apply {
        trace = trace * 10 + 2
        value = value + 4
    }
    return result.value * 100 + trace
}

fun same(input: Box): Boolean = input.apply { value = value + 1 } === input

fun alias(input: Box): Int = input.change { this.value = this.value + 2 }.value

fun custom(input: Box): Int = input.configureValue { value = value + 3 }.value

fun result(input: Box): Int = input.inspectValue { value + 5 }

fun capture(input: Box): Int {
    var observed = 0
    input.apply { observed = value }
    return observed
}

fun nullable(input: Box?): Boolean {
    var observed = false
    val output = input.apply { observed = this == null }
    return observed && output == null
}

fun nonlocal(input: Box, early: Boolean): Int {
    input.apply {
        if (early) return value + 40
        value = value + 1
    }
    return 9
}

fun throwing(input: Box): Int {
    try {
        input.apply {
            value = 17
            throw IllegalArgumentException("inside")
        }
    } catch (error: IllegalArgumentException) {
        return input.value
    }
    return 0
}

fun otherScopes(input: Box): Int = input.evaluate { value } + inside(input) { value } + input.let { it.value }

fun populate(input: Store, providedNames: List<String>, providedCounts: Map<String, Int>, providedNested: List<List<String>>): Store = input.apply {
    names = providedNames
    counts = providedCounts
    nested = providedNested
}

fun clear(input: Store) {
    input.names = null
    input.counts = null
}

fun staticStore(items: List<String>) { Store.global = items }
fun orderedStore() { Store.make().names = Store.data() }
fun receiverFailure() { Store.failMake().names = Store.data() }
fun valueFailure() { Store.make().names = Store.failData() }
