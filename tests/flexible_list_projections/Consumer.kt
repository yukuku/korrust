fun consume(): Boolean {
    val value: List<out String> = listOf("a")
    return readonlyOut(value) == 1 && Holder().accept(value) == 1 && identityOut(value)[0] == "a" && alias(value) == 1
}
