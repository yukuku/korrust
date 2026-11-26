package pkg
fun run(c: Child): String {
    c.value = "changed"
    Base.count = c.frozen
    val f: () -> String? = { c.value }
    return f()!! + Base.count
}
