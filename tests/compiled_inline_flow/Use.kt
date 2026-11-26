package flowuse
import flow.changed
import flow.applyDefault
import flow.changedLong
import flow.doubleSum
import flow.numeric
import flow.dense
import flow.sparse
import flow.materialized
import flow.ranked
import kotlin.math.round

fun integer(a: IntArray): Int = changed(a, 1) { it * 3 }
fun wide(a: LongArray): Long = changedLong(a, 0) { it * 2L }
fun summed(a: DoubleArray): Double = doubleSum(a) { it * 2.0 }
fun converted(a: Double): Double = numeric(a) { it * 3.0 }
fun rounded(a: Double): Double = round(a)
fun switched(a: Int): Int = dense(a) { it + 1 }
fun sparseSwitched(a: Int): Int = sparse(a) { it * 2 }
fun allocated(a: Double): Double = materialized(a) { it[0] + it[1] }
fun compared(a: Double): Int = ranked(a) { it + 1 }
fun nonlocal(a: Int): Int {
    dense(a) { if (it == 20) return 90; it }
    return -1
}

fun capturedDefault(base: Int): Int = applyDefault(5) { it + base }
