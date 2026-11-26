import kotlinx.coroutines.runBlocking
fun bad(): Int = runBlocking { value -> value }