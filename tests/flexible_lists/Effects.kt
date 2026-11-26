import Lists
fun inferred() = Lists.nonNull()
fun inferredNullable() = Lists.nullable(false)
fun inferredPlatform() = Lists.platform(false)
fun inferredReadOnly() = Lists.readOnly()
fun inferredPlatformReadOnly() = Lists.platformReadOnly()
fun inferredPlatformMutable() = Lists.platformMutable()
fun inferredMutable() = Lists.mutable()
fun mutate(): Int {
    val value = Lists.nonNull()
    value.add("a")
    value.add(null)
    value[0] = "b"
    return value.size
}
fun direct(): Boolean = Lists.nonNull().add("a")
fun mutableView(): Int {
    val value: MutableList<CharSequence> = Lists.nonNull()
    value.add("a")
    return value.size
}
fun readonlyView(): Int {
    val value: List<CharSequence> = Lists.nonNull()
    return value.size
}
fun readonlyArgument(value: List<String>): Int = Lists.accept(value)
fun mutableArgument(value: MutableList<CharSequence>): Int = Lists.accept(value)
fun explicitReadonlyArgument(value: List<String>): Int = Lists.readonly(value)
fun explicitMutableArgument(value: MutableList<CharSequence>): Int = Lists.mutableArg(value)
fun nullable(): Int {
    val value = Lists.nullable(false)
    value?.add("a")
    return value?.size ?: -1
}
fun platform(): Int {
    val value = Lists.platform(false)
    value.add("a")
    return value.size
}
