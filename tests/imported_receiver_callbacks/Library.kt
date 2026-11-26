package receivercallbacks
import receiverlibrary.checkpoint
class Lease(val base: Int)
object Activity { var creations = 0 }
fun lease(base: Int): Lease { Activity.creations++; return Lease(base) }
fun <R> within(value: Lease, block: Lease.() -> R): R = value.block()
fun Lease.adjust(value: Int): Int = base + value
suspend fun items(): List<Int> { checkpoint(); return listOf(1, 2, 3) }
suspend fun numbers(): IntArray { checkpoint(); return intArrayOf(4, 5) }
suspend fun letters(): String { checkpoint(); return "abc" }
suspend fun missing(): IntArray? { checkpoint(); return null }
