package stable
import stable.dependency.Box
fun guarded(value: Box): Int = if (value.text != null) value.text.length else -1
fun reversed(value: Box): Int = if (null !== value.text) value.text.length else -1
fun complement(value: Box): Int = if (value.text == null) -1 else value.text.length
fun early(value: Box): Int {
    if (value.text == null) return -1
    return value.text.length
}
fun conjunction(value: Box): Int = if (value.child != null && value.child.text != null) value.child.text.length else -1
fun chain(value: Box): Int {
    if (value.child != null) {
        if (value.child.text != null) return value.child.text.length
    }
    return -1
}
fun shadow(first: Box, second: Box): Int {
    if (first.text != null) {
        val first = second
        return if (first.text != null) first.text.length else -1
    }
    return -2
}
