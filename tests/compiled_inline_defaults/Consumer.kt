package defaults.consumer

import defaults.library.accumulate
import defaults.library.defaultBlock
import defaults.library.guarded
import defaults.library.name
import defaults.library.record
import defaults.library.reifiedBlock
import defaults.library.wide

fun omitted(): Int = accumulate { it + 10 }
fun reordered(): Int = accumulate(next = record(5)) { it }
fun nonlocal(): Int {
    accumulate { return it + 20 }
    return -1
}
fun finalizer(): Int {
    guarded { return 7 }
    return -1
}
fun reified(): String = name<String>()
fun lambdaDefault(): String = defaultBlock("kept")
fun suppliedLambda(): String = reifiedBlock<Int> { "provided" }
fun encoded(): Int = "hé".toByteArray().size

fun wideMask(): Int = wide { it }

class Holder {
    val stored: String = defaultBlock("field")
}

fun fieldDefault(): String = Holder().stored
