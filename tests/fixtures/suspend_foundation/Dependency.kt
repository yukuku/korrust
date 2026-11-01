package suspend_dependency

suspend fun fetch(input: String, callback: suspend (String) -> String): String {
    val value = callback(input)
    return value + "dep"
}
