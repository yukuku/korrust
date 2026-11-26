import kotlinx.coroutines.runBlocking
import receiverlibrary.checkpoint
class Box(val value: Int)
fun use(): Int = runBlocking { Box(checkpoint()).value }
