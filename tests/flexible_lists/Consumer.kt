fun consume(): Boolean {
    val a = inferred()
    a.add("a")
    val b = inferredNullable()
    b?.add("b")
    val c = inferredPlatform()
    c.add("c")
    val d = inferredMutable()
    d.add("d")
    val e = inferredPlatformMutable()
    e.add("e")
    return e.size == 1 && inferredPlatformReadOnly().size == 0 && a.size == 1 && b?.size == 1 && c.size == 1 && d.size == 1 && inferredReadOnly().size == 0
}
