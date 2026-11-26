import assign.Leaf
import assign.Lookup
import assign.Node

fun literal(): Int {
    var value: String? = null
    value = "abc"
    return value.length
}
fun replace(): Int {
    var value: String? = null
    value = "a"
    val first = value.length
    value = "abcd"
    return first + value.length
}
fun subtype(): Int {
    var value: Node? = null
    value = Leaf(7)
    return value.inputType
}
fun platform(lookup: Lookup, kind: Int): Int {
    var value: Leaf? = null
    value = lookup.leaf(kind)
    value.inputType = 3
    return value.inputType
}
fun generic(lookup: Lookup, kind: Int): Int {
    var value: Leaf? = null
    value = lookup.find(kind)
    value.inputType = 4
    return value.inputType
}
fun platformRead(lookup: Lookup, kind: Int): Leaf? {
    var value: Node? = null
    value = lookup.leaf(kind)
    val result: Leaf? = value
    return result
}
fun readonlyCapture(lookup: Lookup, kind: Int): Int {
    var value: Leaf? = null
    value = lookup.find(kind)
    val read: () -> Leaf? = { value }
    value.inputType = 5
    return read()!!.inputType
}
fun beforeChangingCapture(): Int {
    var value: String? = null
    value = "abc"
    val size = value.length
    val change: () -> Unit = { value = null }
    return size
}
fun readonlyThenAssign(): Int {
    var value: String? = null
    val read: () -> String? = { value }
    value = "abc"
    return value.length + read()!!.length
}
fun shadowing(): Int {
    var value: String? = null
    value = "abc"
    val read: () -> String? = {
        var value: String? = null
        value = "other"
        value
    }
    return value.length + read()!!.length
}
fun branches(flag: Boolean): Int {
    var value: String? = null
    if (flag) { value = "a" } else { value = "abcd" }
    return value.length
}
fun checked(input: String?): Int {
    var value: String? = null
    value = input
    if (value != null) return value.length
    return -1
}
fun loop(count: Int): Int {
    var value: String? = null
    var size = 0
    for (i in 0 until count) {
        value = "a"
        size += value.length
    }
    return size
}
fun readonlyLoop(count: Int): Int {
    var value: String? = null
    value = "abc"
    val read: () -> String? = { value }
    var size = 0
    for (i in 0 until count) { size += value.length }
    return size + read()!!.length
}
