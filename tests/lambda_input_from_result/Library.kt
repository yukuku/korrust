package checks
@Suppress("UNCHECKED_CAST")
fun <T : Any> read(value: Any, acceptable: (T) -> Boolean): T {
    val typed = value as T
    check(acceptable(typed))
    return typed
}
