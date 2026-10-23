package korrust.probes

import yuku.alkitab.util.IntArrayList

fun listSize(list: IntArrayList): Int = list.size()
fun listBuffer(list: IntArrayList): IntArray = list.buffer()
fun inferredBuffer(list: IntArrayList, other: IntArrayList): Int {
    val buffer = list.buffer()
    other.clear()
    return buffer.size
}
fun explicitBuffer(list: IntArrayList, other: IntArrayList): Int {
    val buffer: IntArray = list.buffer()
    other.clear()
    return buffer.size
}
fun copiedBuffer(list: IntArrayList, other: IntArrayList): IntArray {
    val buffer = list.buffer()
    other.clear()
    return buffer.copyInto(IntArray(1), 0, 0, 1)
}
fun bufferSize(list: IntArrayList): Int = list.buffer().size
fun mutate(list: IntArrayList, value: Int) {
    list.add(value)
    list.set(0, value)
    list.size()
}
fun emptyList(list: IntArrayList) { list.clear(); return }
fun IntArrayList.explicitSize(): Int = this.size()
fun allocate(size: Int): IntArray = IntArray(size)
fun copy(source: IntArray, destination: IntArray, offset: Int, start: Int, end: Int): IntArray = source.copyInto(destination, offset, start, end)
fun orderedCopy(source: IntArrayList, destination: IntArrayList, indices: IntArrayList): IntArray = source.buffer().copyInto(destination.buffer(), indices.get(0), indices.get(1), indices.get(2))
fun directCopyOrder(list: IntArrayList, other: IntArrayList): IntArray = list.buffer().copyInto(IntArray(other.size()), 0, 0, 0)
fun inferredCopyOrder(list: IntArrayList, other: IntArrayList): IntArray {
    val buffer = list.buffer()
    return buffer.copyInto(IntArray(other.size()), 0, 0, 0)
}
