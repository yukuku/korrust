package iteration
import iteration.dep.External
import iteration.dep.ExtensionBox
import iteration.dep.Boundary
import iteration.dep.iterator
import iteration.dep.rangeUntil

class Cursor(var current: Int, val end: Int) {
    operator fun hasNext(): Boolean { Trace.record("h", 0); return current < end }
    operator fun next(): Int { Trace.record("n", 0); val result = current; current += 1; return result }
}
class Sequence(val end: Int) {
    operator fun iterator(): Cursor { Trace.record("i", 0); return Cursor(0, end) }
}
fun fresh(): Sequence { Trace.record("f", 0); return Sequence(5) }
fun custom(): Int {
    var sum = 0
    outer@ for (n in fresh()) {
        if (n == 1) continue@outer
        if (n == 3) break@outer
        sum += n
    }
    return sum
}
data class Entry(val left: Int, val right: Int)
class EntryCursor(var current: Int) {
    operator fun hasNext(): Boolean = current < 3
    operator fun next(): Entry { current += 1; return Entry(current, current * 10) }
}
class Entries { operator fun iterator(): EntryCursor = EntryCursor(0) }
fun destructured(): Int { var sum = 0; for ((a, b) in Entries()) { sum += a + b }; return sum }
fun compiled(): Long { var sum = 0L; for (n in External(4)) { sum += n }; return sum }
fun extension(): Long { var sum = 0L; for (n in ExtensionBox(4)) { sum += n }; return sum }
class SourceBox(val end: Int)
operator fun SourceBox.iterator(): Cursor = Cursor(0, end)
fun sourceExtension(): Int { var sum = 0; for (n in SourceBox(4)) { sum += n }; return sum }
class Edge(val value: Int) {
    operator fun rangeUntil(other: Edge): Sequence { Trace.record("r", 0); return Sequence(other.value - value) }
}
fun memberRange(): Int { var sum = 0; for (n in Edge(2)..<Edge(6)) { sum += n }; return sum }
fun compiledRange(): Long { var sum = 0L; for (n in Boundary(2)..<Boundary(6)) { sum += n }; return sum }
class Endpoint(val value: Int)
operator fun Endpoint.rangeUntil(other: Endpoint): Sequence = Sequence(other.value - value)
fun extensionRange(): Int { var sum = 0; for (n in Endpoint(2)..<Endpoint(6)) { sum += n }; return sum }
fun intRange(start: Int, end: Int): IntRange = start..<end
fun longRange(start: Long, end: Long): LongRange = start..<end
fun charRange(start: Char, end: Char): CharRange = start..<end
fun rangeEvaluation(): IntRange = Trace.record("a", 2)..<Trace.record("b", 5)
fun intSum(start: Int, end: Int): Int { var sum = 0; for (n in start..<end) { sum += n }; return sum }
fun longSum(start: Long, end: Long): Long { var sum = 0L; for (n in start..<end) { sum += n }; return sum }
fun charSum(start: Char, end: Char): Int { var sum = 0; for (n in start..<end) { sum += n.code }; return sum }
class NullableCursor(var current: Int) {
    operator fun hasNext(): Boolean = current < 2
    operator fun next(): String? { current += 1; return if (current == 1) null else "ok" }
}
class Nullables { operator fun iterator(): NullableCursor = NullableCursor(0) }
fun nullableElements(): String { var result = ""; for (value in Nullables()) { result += value ?: "null" }; return result }
class ThrowingCursor(var current: Int) {
    operator fun hasNext(): Boolean { if (current == 1) throw IllegalStateException("hasNext"); return true }
    operator fun next(): Int { current += 1; return current }
}
class ThrowingValues { operator fun iterator(): ThrowingCursor = ThrowingCursor(0) }
fun protocolFailure(): Int { var sum = 0; for (value in ThrowingValues()) { sum += value }; return sum }
fun nestedLabels(): Int {
    var count = 0
    outer@ for (a in Sequence(3)) {
        for (b in Sequence(3)) {
            if (b == 1) continue@outer
            count += 1
        }
    }
    return count
}
operator fun External.iterator(): Cursor = Cursor(0, 1)

fun initializerLabel(flag: Boolean): Int {
    var count = 0
    outer@ for (value in if (flag) { var n = 0; while (n < 2) { n += 1 }; Sequence(3) } else Sequence(3)) {
        count += 1
        if (value < 2) continue@outer
    }
    return count
}
