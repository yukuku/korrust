package model
class Row(val n: Int?, val text: String?)
fun Row.count(): Int {
    if (n == null) return 0
    if (text != null) return n + text.length
    return n
}
fun explicit(r: Row): Int {
    if (r.n != null) return r.n
    return -1
}
