import referencevarargs.*
fun consume(): Int {
    val input = arrayOf("a", "bb")
    val projected = expose(*input)
    val callback = capture(*projected)
    return join(*projected).length + generic("first", "next").length + projection(input) + Holder(3).count(*projected) + callback().length
}
