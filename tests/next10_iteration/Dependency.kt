package iteration.dep
class ExternalCursor(var current: Int, val end: Int) {
    operator fun hasNext(): Boolean = current < end
    operator fun next(): Long { val result = current; current += 1; return result.toLong() }
}
class External(val end: Int) {
    operator fun iterator(): ExternalCursor = ExternalCursor(0, end)
}
class ExtensionBox(val end: Int)
operator fun ExtensionBox.iterator(): ExternalCursor = ExternalCursor(0, end)
class Boundary(val end: Int)
operator fun Boundary.rangeUntil(other: Boundary): External = External(other.end - end)
