package suspend_dependency

fun interface External {
    suspend fun read(input: String): String?
}
