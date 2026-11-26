import kotlinx.coroutines.runBlocking
fun bad(): Int = runBlocking<Int> { "wrong" }